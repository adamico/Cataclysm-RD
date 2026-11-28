// Fields spike (issue #14) — recorder, Observable dump and soak runner (C++ side).
//
// Built from untouched BN sources (Baseline e7b4976): it links every BN object
// except rng.cpp.o and main.cpp.o, plus rng_recording.cpp and this file.
// It loads the core Content headlessly (like BN's tests/test_main.cpp does),
// builds a small region of submaps in a private dimension that is never inside
// the Reality bubble, seeds a field layout, and runs BN's own
// process_fields_in_submap() for N turns.
//
//   fields_recorder run --bn <bn-src-dir> --user-dir <scratch> --layout <file>
//                       --seeds 1,2,3 --turns N --out-dir <dir> [--record]
//
// Per seed it writes <out-dir>/seed-<s>.dump.jsonl (Observable dump after the
// last turn, plus turn 0) and, with --record, <out-dir>/seed-<s>.case.jsonl.
// Formats: spike/recorder/CASE_FORMAT.md and spike/recorder/DUMP_FORMAT.md.

#include "draw_recorder.h"

#include "action_time_scale.h"
#include "avatar.h"
#include "cached_options.h"
#include "calendar.h"
#include "color.h"
#include "debug.h"
#include "filesystem.h"
#include "game.h"
#include "init.h"
#include "language.h"
#include "loading_ui.h"
#include "map/field.h"
#include "map/field_type.h"
#include "map/map.h"
#include "map/mapbuffer.h"
#include "map/mapbuffer_registry.h"
#include "map/mapdata.h"
#include "map/submap.h"
#include "map/submap_fields.h"
#include "mod_manager.h"
#include "options.h"
#include "path_info.h"
#include "rng.h"
#include "type_id.h"
#include "weather/weather.h"
#include "worldfactory.h"

#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <string>
#include <tuple>
#include <vector>

namespace
{

// Private dimension: never requested by the Reality bubble, so
// process_fields_in_submap() treats the region as "outside the bubble"
// (no transparency/seen cache writes into the primary map).
const dimension_id SPIKE_DIM( "spike_fields" );
// Region origin, in absolute submap coordinates.
const tripoint_abs_sm ORIGIN{ 200, 200, 0 };

struct layout {
    int w = 1; // region width, submaps
    int h = 1; // region height, submaps
    std::string default_ter = "t_dirt";
    struct ter_cell {
        int x, y;
        std::string id;
    };
    struct fld_cell {
        int x, y;
        std::string id;
        int intensity;
        int age_turns;
    };
    std::vector<ter_cell> ters;
    std::vector<fld_cell> flds;
};

// Layout text format (one directive per line, '#' comments):
//   region <w> <h>                         region size in submaps (default 1 1)
//   ter all <ter_id>                       default terrain (default t_dirt)
//   ter <x> <y> <ter_id>                   x,y in region tile coords (0..12*w-1)
//   field <x> <y> <fd_id> <intensity> <age_turns>
layout read_layout( const std::string &path )
{
    std::ifstream in( path );
    if( !in ) {
        throw std::runtime_error( "cannot open layout " + path );
    }
    layout L;
    std::string line;
    while( std::getline( in, line ) ) {
        const auto hash = line.find( '#' );
        if( hash != std::string::npos ) {
            line.erase( hash );
        }
        std::istringstream ss( line );
        std::string kw;
        if( !( ss >> kw ) ) {
            continue;
        }
        if( kw == "region" ) {
            ss >> L.w >> L.h;
        } else if( kw == "ter" ) {
            std::string a;
            ss >> a;
            if( a == "all" ) {
                ss >> L.default_ter;
            } else {
                layout::ter_cell c{ std::stoi( a ), 0, "" };
                ss >> c.y >> c.id;
                L.ters.push_back( c );
            }
        } else if( kw == "field" ) {
            layout::fld_cell c{};
            ss >> c.x >> c.y >> c.id >> c.intensity >> c.age_turns;
            L.flds.push_back( c );
        } else {
            throw std::runtime_error( "bad layout directive: " + kw );
        }
    }
    return L;
}

void init_headless( const std::string &bn_dir, const std::string &user_dir )
{
    // Mirrors init_global_game_state() in BN tests/test_main.cpp.
    test_mode = true;
    setupDebug( DebugOutput::std_err );

    if( !remove_tree( user_dir ) || !assure_dir_exist( user_dir ) ) {
        throw std::runtime_error( "cannot reset user dir " + user_dir );
    }
    PATH_INFO::init_base_path( bn_dir + "/" );
    PATH_INFO::init_user_dir( user_dir + "/" );
    PATH_INFO::set_standard_filenames();
    PATH_INFO::set_config_dir( PATH_INFO::user_dir() + "config/" );
    assure_dir_exist( PATH_INFO::config_dir() );
    assure_dir_exist( PATH_INFO::savedir() );
    assure_dir_exist( PATH_INFO::templatedir() );

    init_language_system();
    get_options().init();
    get_options().load();
    init_colors();

    g = std::make_unique<game>();
    g->new_game = true;
    g->load_static_data();

    world_generator->set_active_world( nullptr );
    world_generator->init();
    WORLDINFO *world = world_generator->make_new_world( { mod_id( "bn" ) } );
    if( world == nullptr ) {
        throw std::runtime_error( "cannot make world" );
    }
    world_generator->set_active_world( world );

    calendar::set_eternal_season( get_option<bool>( "ETERNAL_SEASON" ) );
    calendar::set_season_length( get_option<int>( "SEASON_LENGTH" ) );

    loading_ui ui( false );
    init::load_world_modfiles( ui, g->get_active_world(), SAVE_ARTIFACTS );

    g->u = avatar();
    g->u.create( character_type::NOW );
    g->m = map();
    disable_mapgen = true;
    g->m.load( g->m.get_abs_sub(), false );
    get_weather().update_weather();
}

std::vector<std::pair<tripoint_abs_sm, submap *>> build_region( const layout &L )
{
    MAPBUFFER_REGISTRY.unload_dimension( SPIKE_DIM );
    auto &mb = MAPBUFFER_REGISTRY.get( SPIKE_DIM );
    const ter_id def = ter_str_id( L.default_ter ).id();
    std::vector<std::pair<tripoint_abs_sm, submap *>> region;
    for( int sy = 0; sy < L.h; ++sy ) {
        for( int sx = 0; sx < L.w; ++sx ) {
            const tripoint_abs_sm pos( ORIGIN.raw() + tripoint( sx, sy, 0 ) );
            auto sm = std::make_unique<submap>( pos, SPIKE_DIM );
            mb.add_submap( pos, sm );
            submap *p = mb.lookup_submap_in_memory( pos );
            for( int x = 0; x < SEEX; ++x ) {
                for( int y = 0; y < SEEY; ++y ) {
                    p->set_ter( point_sm_ms( x, y ), def );
                }
            }
            region.emplace_back( pos, p );
        }
    }
    auto at = [&]( int x, int y ) -> std::pair<submap *, point_sm_ms> {
        const int sx = x / SEEX;
        const int sy = y / SEEY;
        if( x < 0 || y < 0 || sx >= L.w || sy >= L.h )
        {
            throw std::runtime_error( "layout cell outside region" );
        }
        return { region[sy * L.w + sx].second, point_sm_ms( x % SEEX, y % SEEY ) };
    };
    for( const auto &c : L.ters ) {
        auto [sm, p] = at( c.x, c.y );
        sm->set_ter( p, ter_str_id( c.id ).id() );
    }
    for( const auto &c : L.flds ) {
        auto [sm, p] = at( c.x, c.y );
        // Same bookkeeping as plant_fire() in BN tests/simulation_test.cpp.
        if( sm->get_field( p ).add_field( field_type_str_id( c.id ).id(), c.intensity,
                                          time_duration::from_turns( c.age_turns ) ) ) {
            ++sm->field_count;
            sm->field_cache.push_back( p );
            sm->is_uniform = false;
        }
    }
    return region;
}

// Observable dump: one line per field entry, sorted by (y, x, type id).
void write_dump( std::FILE *out, const layout &L,
                 const std::vector<std::pair<tripoint_abs_sm, submap *>> &region, unsigned seed,
                 int turn )
{
    struct row {
        int y, x;
        std::string t;
        int n, age;
        bool alive;
    };
    std::vector<row> rows;
    for( int i = 0; i < static_cast<int>( region.size() ); ++i ) {
        const int sx = i % L.w;
        const int sy = i / L.w;
        const submap &sm = *region[i].second;
        for( int x = 0; x < SEEX; ++x ) {
            for( int y = 0; y < SEEY; ++y ) {
                for( const auto &kv : sm.get_field( point_sm_ms( x, y ) ) ) {
                    const field_entry &e = kv.second;
                    rows.push_back( { sy * SEEY + y, sx * SEEX + x, e.get_field_type().id().str(),
                                      e.get_field_intensity(), to_turns<int>( e.get_field_age() ),
                                      e.is_field_alive() } );
                }
            }
        }
    }
    std::sort( rows.begin(), rows.end(), []( const row & a, const row & b ) {
        return std::tie( a.y, a.x, a.t ) < std::tie( b.y, b.x, b.t );
    } );
    std::fprintf( out,
                  "{\"k\":\"dump\",\"format\":\"bn-fields-dump/1\",\"seed\":%u,\"turn\":%d,"
                  "\"region\":[%d,%d],\"entries\":%zu}\n",
                  seed, turn, L.w, L.h, rows.size() );
    for( const row &r : rows ) {
        std::fprintf( out, "{\"k\":\"f\",\"x\":%d,\"y\":%d,\"t\":\"%s\",\"n\":%d,\"age\":%d,"
                      "\"alive\":%s}\n",
                      r.x, r.y, r.t.c_str(), r.n, r.age, r.alive ? "true" : "false" );
    }
}

std::string arg( std::vector<std::string> &args, const std::string &key, const std::string &def,
                 bool required = false )
{
    for( size_t i = 0; i + 1 < args.size(); ++i ) {
        if( args[i] == key ) {
            std::string v = args[i + 1];
            args.erase( args.begin() + i, args.begin() + i + 2 );
            return v;
        }
    }
    if( required ) {
        throw std::runtime_error( "missing " + key );
    }
    return def;
}

bool flag( std::vector<std::string> &args, const std::string &key )
{
    auto it = std::find( args.begin(), args.end(), key );
    if( it == args.end() ) {
        return false;
    }
    args.erase( it );
    return true;
}

int run( std::vector<std::string> args )
{
    const std::string bn_dir = arg( args, "--bn", "", true );
    const std::string user_dir = arg( args, "--user-dir", "", true );
    const std::string layout_path = arg( args, "--layout", "", true );
    const std::string seeds_s = arg( args, "--seeds", "1" );
    const int turns = std::stoi( arg( args, "--turns", "10" ) );
    const std::string out_dir = arg( args, "--out-dir", "", true );
    const bool record = flag( args, "--record" );
    if( !args.empty() ) {
        throw std::runtime_error( "unknown argument " + args.front() );
    }

    std::vector<unsigned> seeds;
    {
        std::stringstream ss( seeds_s );
        std::string s;
        while( std::getline( ss, s, ',' ) ) {
            seeds.push_back( static_cast<unsigned>( std::stoul( s ) ) );
        }
    }
    const layout L = read_layout( layout_path );
    init_headless( bn_dir, user_dir );
    assure_dir_exist( out_dir );

    for( const unsigned seed : seeds ) {
        if( seed == 0 ) {
            throw std::runtime_error( "seed 0 is ignored by rng_set_engine_seed; use >0" );
        }
        // Fixed start time for every seed: one hour past turn_zero, single-turn ticks.
        calendar::turn = calendar::turn_zero + 1_hours;
        action_time_scale::set_calendar_turns_this_tick( 1 );
        auto region = build_region( L );
        auto &mb = MAPBUFFER_REGISTRY.get( SPIKE_DIM );

        const std::string stem = out_dir + "/seed-" + std::to_string( seed );
        std::FILE *dump = std::fopen( ( stem + ".dump.jsonl" ).c_str(), "w" );
        write_dump( dump, L, region, seed, 0 );

        if( record ) {
            draw_recorder::begin( stem + ".case.jsonl",
                                  "\"case\":\"soak\",\"seed\":" + std::to_string( seed ) +
                                  ",\"turns\":" + std::to_string( turns ) + ",\"layout\":\"" +
                                  layout_path.substr( layout_path.find_last_of( '/' ) + 1 ) + "\"" );
        }
        rng_set_engine_seed( seed );
        for( int t = 1; t <= turns; ++t ) {
            if( record ) {
                draw_recorder::mark( "turn", "\"turn\":" + std::to_string( t ) );
            }
            // Fixed processing order: row-major over the region.
            for( size_t i = 0; i < region.size(); ++i ) {
                if( record ) {
                    draw_recorder::mark( "submap", "\"index\":" + std::to_string( i ) );
                }
                if( region[i].second->field_count > 0 ) {
                    process_fields_in_submap( SPIKE_DIM, *region[i].second, region[i].first, mb );
                }
            }
            calendar::turn += 1_turns;
        }
        if( record ) {
            draw_recorder::end();
        }
        write_dump( dump, L, region, seed, turns );
        std::fclose( dump );
        std::printf( "seed %u: %d turns -> %s.dump.jsonl%s\n", seed, turns, stem.c_str(),
                     record ? " (+ case)" : "" );
    }
    MAPBUFFER_REGISTRY.unload_dimension( SPIKE_DIM );
    return debug_has_error_been_observed() ? 2 : 0;
}

} // namespace

int main( int argc, char **argv )
{
    std::vector<std::string> args( argv + 1, argv + argc );
    if( args.empty() || args.front() != "run" ) {
        std::fprintf( stderr,
                      "usage: fields_recorder run --bn <bn-src> --user-dir <dir> --layout <file>\n"
                      "                          --seeds 1,2,3 --turns N --out-dir <dir> [--record]\n" );
        return 1;
    }
    args.erase( args.begin() );
    int rc;
    try {
        rc = run( args );
    } catch( const std::exception &e ) {
        std::fprintf( stderr, "fields_recorder: %s\n", e.what() );
        rc = 1;
    }
    // Skip static destructors: at exit BN destroys DynamicDataLoader's Lua state
    // and then a Lua hook cache that points into it ("free(): invalid pointer"
    // in cata::lua_state_deleter). All output files are closed by now.
    std::fflush( nullptr );
    std::_Exit( rc );
}
