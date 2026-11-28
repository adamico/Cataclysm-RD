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
#include "creature.h"
#include "effect.h"
#include "enum_conversions.h"
#include "fire.h"
#include "monster.h"
#include "mtype.h"
#include "teleport.h"
#include "item.h"
#include "itype.h"
#include "material.h"
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
#include <cinttypes>
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
    int zlo = 0; // z-levels of the region (session 3)
    int zhi = 0;
    int clock = 3600; // calendar::turn = turn_zero + clock turns at turn 1
    std::string default_ter = "t_dirt";
    struct ter_cell {
        int x, y;
        std::string id;
        int z = 0;
    };
    struct fld_cell {
        int x, y;
        std::string id;
        int intensity;
        int age_turns;
        int z = 0;
    };
    struct rad_cell {
        int x, y, rad, z = 0;
    };
    std::vector<ter_cell> ters;
    std::vector<ter_cell> furns;
    std::vector<ter_cell> items;
    std::vector<rad_cell> rads;
    // Session 3: `monster <x> <y> <mon_id>` makes this a creature case: the
    // layout is built in the bubble map (g->m) and each turn runs
    // map::creature_in_field on the monster instead of processing fields.
    std::vector<ter_cell> mons;
    std::vector<fld_cell> flds;
    int nz() const {
        return zhi - zlo + 1;
    }
};

// Optional trailing z on a directive.
int opt_z( std::istringstream &ss )
{
    int z = 0;
    if( !( ss >> z ) ) {
        z = 0;
    }
    return z;
}

// Layout text format (one directive per line, '#' comments):
//   region <w> <h>                         region size in submaps (default 1 1)
//   ter all <ter_id>                       default terrain (default t_dirt)
//   ter <x> <y> <ter_id>                   x,y in region tile coords (0..12*w-1)
//   field <x> <y> <fd_id> <intensity> <age_turns> [z]
// Session 3 additions (all take an optional trailing z, default 0):
//   clock <turns>                          start time = turn_zero + turns (default 3600 = 1 h)
//   zlevels <lo> <hi>                      z-levels of the region (default 0 0)
//   ter <x> <y> <ter_id> [z]
//   furn <x> <y> <furn_id> [z]
//   item <x> <y> <itype_id> [z]            one item, spawned at the start time
//   rad <x> <y> <radiation> [z]
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
        } else if( kw == "clock" ) {
            ss >> L.clock;
        } else if( kw == "zlevels" ) {
            ss >> L.zlo >> L.zhi;
        } else if( kw == "ter" ) {
            std::string a;
            ss >> a;
            if( a == "all" ) {
                ss >> L.default_ter;
            } else {
                layout::ter_cell c{ std::stoi( a ), 0, "" };
                ss >> c.y >> c.id;
                c.z = opt_z( ss );
                L.ters.push_back( c );
            }
        } else if( kw == "furn" || kw == "item" ) {
            layout::ter_cell c{};
            ss >> c.x >> c.y >> c.id;
            c.z = opt_z( ss );
            ( kw == "furn" ? L.furns : L.items ).push_back( c );
        } else if( kw == "monster" ) {
            layout::ter_cell c{};
            ss >> c.x >> c.y >> c.id;
            L.mons.push_back( c );
        } else if( kw == "rad" ) {
            layout::rad_cell c{};
            ss >> c.x >> c.y >> c.rad;
            c.z = opt_z( ss );
            L.rads.push_back( c );
        } else if( kw == "field" ) {
            layout::fld_cell c{};
            ss >> c.x >> c.y >> c.id >> c.intensity >> c.age_turns;
            c.z = opt_z( ss );
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
    // Order: z ascending, then row-major (y, then x) within a z-level.
    for( int z = L.zlo; z <= L.zhi; ++z ) {
        for( int sy = 0; sy < L.h; ++sy ) {
            for( int sx = 0; sx < L.w; ++sx ) {
                const tripoint_abs_sm pos( ORIGIN.raw() + tripoint( sx, sy, z ) );
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
    }
    auto at = [&]( int x, int y, int z ) -> std::pair<submap *, point_sm_ms> {
        const int sx = x / SEEX;
        const int sy = y / SEEY;
        if( x < 0 || y < 0 || sx >= L.w || sy >= L.h || z < L.zlo || z > L.zhi )
        {
            throw std::runtime_error( "layout cell outside region" );
        }
        return { region[( z - L.zlo ) * L.w * L.h + sy * L.w + sx].second, point_sm_ms( x % SEEX, y % SEEY ) };
    };
    for( const auto &c : L.ters ) {
        auto [sm, p] = at( c.x, c.y, c.z );
        sm->set_ter( p, ter_str_id( c.id ).id() );
    }
    for( const auto &c : L.furns ) {
        auto [sm, p] = at( c.x, c.y, c.z );
        sm->set_furn( p, furn_str_id( c.id ).id() );
    }
    for( const auto &c : L.rads ) {
        auto [sm, p] = at( c.x, c.y, c.z );
        sm->set_radiation( p, c.rad );
    }
    for( const auto &c : L.items ) {
        auto [sm, p] = at( c.x, c.y, c.z );
        sm->get_items( p ).push_back( item::spawn( itype_id( c.id ), calendar::turn ) );
        sm->is_uniform = false;
    }
    for( const auto &c : L.flds ) {
        auto [sm, p] = at( c.x, c.y, c.z );
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
        int z, y, x;
        std::string t;
        int n, age;
        bool alive;
    };
    std::vector<row> rows;
    for( int i = 0; i < static_cast<int>( region.size() ); ++i ) {
        const int sx = i % L.w;
        const int sy = ( i / L.w ) % L.h;
        const int sz = L.zlo + i / ( L.w * L.h );
        const submap &sm = *region[i].second;
        for( int x = 0; x < SEEX; ++x ) {
            for( int y = 0; y < SEEY; ++y ) {
                for( const auto &kv : sm.get_field( point_sm_ms( x, y ) ) ) {
                    const field_entry &e = kv.second;
                    rows.push_back( { sz, sy * SEEY + y, sx * SEEX + x, e.get_field_type().id().str(),
                                      e.get_field_intensity(), to_turns<int>( e.get_field_age() ),
                                      e.is_field_alive() } );
                }
            }
        }
    }
    std::sort( rows.begin(), rows.end(), []( const row & a, const row & b ) {
        return std::tie( a.z, a.y, a.x, a.t ) < std::tie( b.z, b.y, b.x, b.t );
    } );
    // Single-z regions keep the session 1 format byte for byte; multi-z regions
    // add "z" after "y" (DUMP_FORMAT.md).
    const bool multi_z = L.nz() > 1;
    std::fprintf( out,
                  "{\"k\":\"dump\",\"format\":\"bn-fields-dump/1\",\"seed\":%u,\"turn\":%d,"
                  "\"region\":[%d,%d],\"entries\":%zu}\n",
                  seed, turn, L.w, L.h, rows.size() );
    for( const row &r : rows ) {
        const std::string zs = multi_z ? ",\"z\":" + std::to_string( r.z ) : "";
        std::fprintf( out, "{\"k\":\"f\",\"x\":%d,\"y\":%d%s,\"t\":\"%s\",\"n\":%d,\"age\":%d,"
                      "\"alive\":%s}\n",
                      r.x, r.y, zs.c_str(), r.t.c_str(), r.n, r.age, r.alive ? "true" : "false" );
    }
}

// ---- session 3: case context lines -----------------------------------------
// The Rust replay needs what BN knows that the layout file does not say: the
// terrain/furniture flags and move costs (Rust's catalogue is a stub) and the
// item properties the Fields rules read. Written as `mark` lines at the start
// of a case (CASE_FORMAT.md, "Session 3 additions").

std::string json_flags_ter( const ter_t &t )
{
    std::string out;
    auto add = [&]( bool on, const char *name ) {
        if( on ) {
            out += std::string( out.empty() ? "" : "," ) + "\"" + name + "\"";
        }
    };
    add( t.has_flag( TFLAG_FLAMMABLE ), "FLAMMABLE" );
    add( t.has_flag( TFLAG_FLAMMABLE_ASH ), "FLAMMABLE_ASH" );
    add( t.has_flag( TFLAG_FLAMMABLE_HARD ), "FLAMMABLE_HARD" );
    add( t.has_flag( TFLAG_FIRE_CONTAINER ), "FIRE_CONTAINER" );
    add( t.has_flag( TFLAG_NO_FLOOR ), "NO_FLOOR" );
    add( t.has_flag( TFLAG_SEALED ), "SEALED" );
    add( t.has_flag( TFLAG_ALLOW_FIELD_EFFECT ), "ALLOW_FIELD_EFFECT" );
    add( t.has_flag( TFLAG_SWIMMABLE ), "SWIMMABLE" );
    add( t.has_flag( TFLAG_PERMEABLE ), "PERMEABLE" );
    add( t.has_flag( "FUNGUS" ), "FUNGUS" );
    return "[" + out + "]";
}
std::string json_flags_furn( const furn_t &t )
{
    std::string out;
    auto add = [&]( bool on, const char *name ) {
        if( on ) {
            out += std::string( out.empty() ? "" : "," ) + "\"" + name + "\"";
        }
    };
    add( t.has_flag( TFLAG_FLAMMABLE ), "FLAMMABLE" );
    add( t.has_flag( TFLAG_FLAMMABLE_ASH ), "FLAMMABLE_ASH" );
    add( t.has_flag( TFLAG_FLAMMABLE_HARD ), "FLAMMABLE_HARD" );
    add( t.has_flag( TFLAG_FIRE_CONTAINER ), "FIRE_CONTAINER" );
    add( t.has_flag( TFLAG_NO_FLOOR ), "NO_FLOOR" );
    add( t.has_flag( TFLAG_SEALED ), "SEALED" );
    add( t.has_flag( TFLAG_ALLOW_FIELD_EFFECT ), "ALLOW_FIELD_EFFECT" );
    add( t.has_flag( TFLAG_SWIMMABLE ), "SWIMMABLE" );
    add( t.has_flag( TFLAG_PERMEABLE ), "PERMEABLE" );
    add( t.has_flag( "FUNGUS" ), "FUNGUS" );
    return "[" + out + "]";
}

void write_defs( const layout &L,
                 const std::vector<std::pair<tripoint_abs_sm, submap *>> &region )
{
    std::vector<std::string> ters{ "t_null", "t_dirt", "t_pit", L.default_ter };
    std::vector<std::string> furns{ "f_null", "f_ash" };
    for( const auto &c : L.ters ) {
        ters.push_back( c.id );
    }
    for( const auto &c : L.furns ) {
        furns.push_back( c.id );
    }
    std::sort( ters.begin(), ters.end() );
    ters.erase( std::unique( ters.begin(), ters.end() ), ters.end() );
    std::sort( furns.begin(), furns.end() );
    furns.erase( std::unique( furns.begin(), furns.end() ), furns.end() );
    for( const auto &id : ters ) {
        const ter_t &t = ter_str_id( id ).obj();
        draw_recorder::mark( "ter_def", "\"id\":\"" + id + "\",\"movecost\":" +
                             std::to_string( t.movecost ) + ",\"flags\":" + json_flags_ter( t ) );
    }
    for( const auto &id : furns ) {
        const furn_t &t = furn_str_id( id ).obj();
        draw_recorder::mark( "furn_def", "\"id\":\"" + id + "\",\"movecost\":" +
                             std::to_string( t.movecost ) + ",\"flags\":" + json_flags_furn( t ) );
    }
    // Items in layout order per tile (= location_vector order).
    for( int i = 0; i < static_cast<int>( region.size() ); ++i ) {
        const int sx = i % L.w;
        const int sy = ( i / L.w ) % L.h;
        const int sz = L.zlo + i / ( L.w * L.h );
        submap &sm = *region[i].second;
        for( int x = 0; x < SEEX; ++x ) {
            for( int y = 0; y < SEEY; ++y ) {
                int idx = 0;
                for( const item *it : sm.get_items( point_sm_ms( x, y ) ) ) {
                    draw_recorder::mark( "item_def",
                                         "\"x\":" + std::to_string( sx * SEEX + x ) + ",\"y\":" +
                                         std::to_string( sy * SEEY + y ) + ",\"z\":" + std::to_string( sz ) +
                                         ",\"index\":" + std::to_string( idx++ ) + ",\"itype\":\"" +
                                         it->typeId().str() + "\",\"weight_g\":" +
                                         std::to_string( units::to_gram( it->weight( false ) ) ) +
                                         ",\"flammable\":" + ( it->flammable() ? "true" : "false" ) +
                                         ",\"rock\":" + ( it->typeId() == itype_id( "rock" ) ? "true" : "false" ) );
                }
            }
        }
    }
}

// Non-field tile state after a turn, for every tile that differs from a
// default tile (default terrain, no furniture, no radiation, no items).
void write_tiles( const layout &L,
                  const std::vector<std::pair<tripoint_abs_sm, submap *>> &region, int turn )
{
    const ter_id def = ter_str_id( L.default_ter ).id();
    for( int i = 0; i < static_cast<int>( region.size() ); ++i ) {
        const int sx = i % L.w;
        const int sy = ( i / L.w ) % L.h;
        const int sz = L.zlo + i / ( L.w * L.h );
        submap &sm = *region[i].second;
        for( int y = 0; y < SEEY; ++y ) {
            for( int x = 0; x < SEEX; ++x ) {
                const point_sm_ms p( x, y );
                const ter_id t = sm.get_ter( p );
                const furn_id f = sm.get_furn( p );
                const int rad = sm.get_radiation( p );
                const size_t n_items = sm.get_items( p ).size();
                if( t == def && f == furn_str_id( "f_null" ).id() && rad == 0 && n_items == 0 ) {
                    continue;
                }
                draw_recorder::mark( "tile", "\"turn\":" + std::to_string( turn ) + ",\"x\":" +
                                     std::to_string( sx * SEEX + x ) + ",\"y\":" + std::to_string( sy * SEEY + y ) +
                                     ",\"z\":" + std::to_string( sz ) + ",\"ter\":\"" + t.id().str() +
                                     "\",\"furn\":\"" + f.id().str() + "\",\"rad\":" + std::to_string( rad ) +
                                     ",\"items\":" + std::to_string( n_items ) );
            }
        }
    }
}

// ---- session 3: creature cases in the bubble map ------------------------------
// The creature rules (map::creature_in_field -> monster_in_field) are map
// members that read the bubble map, so these cases run in g->m: the layout's
// area starts at bubble tile BUB0, z 0. Field processing does not run; one
// "turn" is one call of creature_in_field.
const tripoint_bub_ms BUB0{ 48, 48, 0 };

std::string quote( const std::string &s )
{
    return "\"" + s + "\"";
}

// Every query the Rust Combat port asks of a monster, answered by BN.
void write_creature_def( const monster &z, const layout &L )
{
    std::string tags;
    auto tag = [&]( bool on, const std::string &t ) {
        if( on ) {
            tags += std::string( tags.empty() ? "" : "," ) + quote( t );
        }
    };
    for( const char *f : {
             "WEBWALK", "SLUDGEPROOF", "FIREPROOF", "FIREY", "NO_BREATHE", "SEES", "ELECTRONIC"
         } ) {
        tag( z.has_flag( io::string_to_enum<m_flag>( f ) ), f );
    }
    tag( z.made_of( material_id( "veggy" ) ), "mat:veggy" );
    tag( z.made_of_any( Creature::cmat_flesh ), "grp:Flesh" );
    tag( z.made_of_any( Creature::cmat_fleshnveg ), "grp:FleshNVeg" );
    tag( z.made_of_any( Creature::cmat_flammable ), "grp:Flammable" );
    tag( z.made_of_any( Creature::cmat_flameres ), "grp:Flameres" );
    tag( z.made_of( LIQUID ), "liquid" );
    for( const char *sp : { "FUNGUS", "INSECT", "SPIDER" } ) {
        tag( z.in_species( species_id( sp ) ), std::string( "species:" ) + sp );
    }
    // Field and effect immunities, for every field type and field effect the
    // layout can put on the monster's tile.
    std::vector<std::string> fields;
    for( const auto &c : L.flds ) {
        fields.push_back( c.id );
    }
    std::sort( fields.begin(), fields.end() );
    fields.erase( std::unique( fields.begin(), fields.end() ), fields.end() );
    for( const auto &f : fields ) {
        const field_type &ft = field_type_str_id( f ).obj();
        tag( z.is_immune_field( ft.id ), "immune_field:" + f );
        for( const auto &lvl : ft.intensity_levels ) {
            for( const auto &fe : lvl.field_effects ) {
                tag( z.is_immune_effect( fe.id ), "immune:" + fe.id.str() );
            }
        }
    }
    draw_recorder::mark( "creature_def",
                         "\"id\":" + quote( z.type->id.str() ) + ",\"monster\":true,\"flies\":" +
                         ( z.flies() ? "true" : "false" ) + ",\"digs\":" + ( z.digs() ? "true" : "false" ) +
                         ",\"digging\":" + ( z.digging() ? "true" : "false" ) + ",\"armor_heat_torso\":" +
                         std::to_string( z.get_armor_type( DT_HEAT, bodypart_id( "torso" ) ) ) + ",\"tags\":[" + tags +
                         "]" );
}

void write_bubble_dump( std::FILE *out, const layout &L, unsigned seed, int turn )
{
    map &m = get_map();
    struct row {
        int y, x;
        std::string t;
        int n, age;
        bool alive;
    };
    std::vector<row> rows;
    for( int x = 0; x < L.w * SEEX; ++x ) {
        for( int y = 0; y < L.h * SEEY; ++y ) {
            for( const auto &kv : m.field_at( BUB0 + point( x, y ) ) ) {
                const field_entry &e = kv.second;
                rows.push_back( { y, x, e.get_field_type().id().str(), e.get_field_intensity(),
                                  to_turns<int>( e.get_field_age() ), e.is_field_alive() } );
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

int run_bubble( const layout &L, const std::vector<unsigned> &seeds, int turns,
                const std::string &out_dir, bool record, bool dump_every, const std::string &case_name,
                const std::string &layout_path )
{
    map &m = get_map();
    if( L.mons.size() != 1 || L.nz() != 1 ) {
        throw std::runtime_error( "creature cases take one monster and one z-level" );
    }
    // Keep the avatar out of the area.
    g->u.setpos( BUB0 + tripoint( -20, -20, 0 ) );
    for( const unsigned seed : seeds ) {
        calendar::turn = calendar::turn_zero + time_duration::from_turns( L.clock );
        action_time_scale::set_calendar_turns_this_tick( 1 );
        g->clear_zombies();
        const ter_id def = ter_str_id( L.default_ter ).id();
        for( int x = -1; x <= L.w * SEEX; ++x ) {
            for( int y = -1; y <= L.h * SEEY; ++y ) {
                const tripoint_bub_ms p = BUB0 + point( x, y );
                m.ter_set( p, def );
                m.furn_set( p, f_null );
                std::vector<field_type_id> here;
                for( const auto &kv : m.field_at( p ) ) {
                    here.push_back( kv.first );
                }
                for( const auto &t : here ) {
                    m.remove_field( p, t );
                }
            }
        }
        for( const auto &c : L.ters ) {
            m.ter_set( BUB0 + point( c.x, c.y ), ter_str_id( c.id ).id() );
        }
        // Placed before the fields: place_critter_at refuses a dangerous tile.
        monster *z = g->place_critter_at( mtype_id( L.mons[0].id ), BUB0 + point( L.mons[0].x, L.mons[0].y ) );
        if( z == nullptr ) {
            throw std::runtime_error( "cannot place " + L.mons[0].id );
        }
        z->moves = 0;
        for( const auto &c : L.flds ) {
            m.add_field( BUB0 + point( c.x, c.y ), field_type_str_id( c.id ).id(), c.intensity,
                         time_duration::from_turns( c.age_turns ) );
        }

        const std::string stem = out_dir + "/seed-" + std::to_string( seed );
        std::FILE *dump = std::fopen( ( stem + ".dump.jsonl" ).c_str(), "w" );
        write_bubble_dump( dump, L, seed, 0 );
        if( record ) {
            draw_recorder::begin( stem + ".case.jsonl",
                                  "\"case\":\"" + case_name + "\",\"seed\":" + std::to_string( seed ) +
                                  ",\"turns\":" + std::to_string( turns ) + ",\"layout\":\"" +
                                  layout_path.substr( layout_path.find_last_of( '/' ) + 1 ) + "\",\"rule\":\"creature_in_field\"" );
            write_creature_def( *z, L );
        }
        auto state = [&]( int t ) {
            draw_recorder::mark( "creature", "\"turn\":" + std::to_string( t ) + ",\"hp\":" +
                                 std::to_string( z->get_hp() ) + ",\"moves\":" + std::to_string( z->moves ) +
                                 ",\"dead\":" + ( z->is_dead() ? "true" : "false" ) );
        };
        if( record ) {
            state( 0 );
        }
        rng_set_engine_seed( seed );
        for( int t = 1; t <= turns; ++t ) {
            if( record ) {
                draw_recorder::mark( "turn", "\"turn\":" + std::to_string( t ) );
            }
            if( !z->is_dead() ) {
                m.creature_in_field( *z );
            }
            calendar::turn += 1_turns;
            if( record ) {
                draw_recorder::mark( "end_turn", "\"turn\":" + std::to_string( t ) );
                state( t );
            }
            if( dump_every && t < turns ) {
                write_bubble_dump( dump, L, seed, t );
            }
        }
        if( record ) {
            draw_recorder::end();
        }
        write_bubble_dump( dump, L, seed, turns );
        std::fclose( dump );
        std::printf( "seed %u: %d creature turns -> %s.dump.jsonl\n", seed, turns, stem.c_str() );
        g->clear_zombies();
    }
    return debug_has_error_been_observed() ? 2 : 0;
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
    // Session 3: a snapshot after every turn, not only after the last one.
    const bool dump_every = flag( args, "--dump-every" );
    const std::string case_name = arg( args, "--case", "soak" );
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
    if( !L.mons.empty() ) {
        return run_bubble( L, seeds, turns, out_dir, record, dump_every, case_name, layout_path );
    }

    for( const unsigned seed : seeds ) {
        if( seed == 0 ) {
            throw std::runtime_error( "seed 0 is ignored by rng_set_engine_seed; use >0" );
        }
        // Fixed start time for every seed: one hour past turn_zero, single-turn ticks.
        calendar::turn = calendar::turn_zero + time_duration::from_turns( L.clock );
        action_time_scale::set_calendar_turns_this_tick( 1 );
        auto region = build_region( L );
        auto &mb = MAPBUFFER_REGISTRY.get( SPIKE_DIM );

        const std::string stem = out_dir + "/seed-" + std::to_string( seed );
        std::FILE *dump = std::fopen( ( stem + ".dump.jsonl" ).c_str(), "w" );
        write_dump( dump, L, region, seed, 0 );

        if( record ) {
            draw_recorder::begin( stem + ".case.jsonl",
                                  "\"case\":\"" + case_name + "\",\"seed\":" + std::to_string( seed ) +
                                  ",\"turns\":" + std::to_string( turns ) + ",\"layout\":\"" +
                                  layout_path.substr( layout_path.find_last_of( '/' ) + 1 ) + "\"" );
        }
        if( record ) {
            write_defs( L, region );
            write_tiles( L, region, 0 );
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
            if( record ) {
                draw_recorder::mark( "end_turn", "\"turn\":" + std::to_string( t ) );
                write_tiles( L, region, t );
            }
            if( dump_every && t < turns ) {
                write_dump( dump, L, region, seed, t );
            }
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

// ---- session 3: calls out of Fields, wrapped at link time -------------------
// build.sh links with -Wl,--wrap=<symbol>, so every call from another object
// file (here: map_field.cpp.o) lands in __wrap_<symbol>, which records the
// inputs and outputs around the real call as `call` / `ret` marks. Draws made
// inside the real call sit between the two marks in the case. BN source is
// untouched; the Rust stubs answer from these marks (CASE_FORMAT.md).

extern "C" bool __real__ZN4item4burnER9fire_data( item *self, fire_data &frd );
extern "C" bool __wrap__ZN4item4burnER9fire_data( item *self, fire_data &frd )
{
    const units::mass old_weight = self->weight( false );
    char buf[512];
    std::snprintf( buf, sizeof( buf ),
                   "\"fn\":\"item::burn\",\"itype\":\"%s\",\"weight_g\":%lld,\"fire_intensity\":%d,"
                   "\"contained\":%s,\"fuel_produced\":%.9g,\"smoke_produced\":%.9g",
                   self->typeId().c_str(), static_cast<long long>( units::to_gram( old_weight ) ),
                   frd.fire_intensity, frd.contained ? "true" : "false",
                   static_cast<double>( frd.fuel_produced ), static_cast<double>( frd.smoke_produced ) );
    draw_recorder::mark( "call", buf );
    const bool destroyed = __real__ZN4item4burnER9fire_data( self, frd );
    const units::mass new_weight = destroyed ? 0_gram : self->weight( false );
    // What map_field.cpp's create_burnproducts() will push for this burn (it
    // is file-local there, so it cannot be wrapped). Same arithmetic, read
    // from the same materials; no draws, no items constructed here.
    std::string products;
    int n_products = 0;
    if( old_weight != new_weight ) {
        const std::vector<material_id> all_mats = self->made_of();
        if( !all_mats.empty() ) {
            const units::mass by_weight = ( old_weight - new_weight ) / all_mats.size();
            for( const material_id &mat : all_mats ) {
                for( const auto &bp : mat->burn_products() ) {
                    const itype_id id = bp.first;
                    if( self->typeId() == id ) {
                        continue;
                    }
                    const int n = static_cast<int>( std::floor( bp.second * ( by_weight / id->weight ) ) );
                    if( n > 0 ) {
                        // A fresh item::spawn(id): count-by-charges items get
                        // charges_default() charges, so weigh that many.
                        // item::flammable() for a fresh item of this type.
                        int flammability = 0;
                        bool immune = false;
                        for( const material_id &m : id->materials ) {
                            const mat_burn_data &bd = m->burn_data( 1 );
                            immune = immune || bd.immune;
                            flammability += bd.fuel;
                        }
                        const bool flam = !id->materials.empty() && !immune && flammability > 0;
                        products += std::string( products.empty() ? "" : "," ) + "{\"itype\":\"" + id.str() +
                                    "\",\"weight_g\":" + std::to_string( units::to_gram( id->weight ) *
                                            ( id->count_by_charges() ? id->charges_default() : 1 ) ) +
                                    ",\"flammable\":" + ( flam ? "true" : "false" ) + "}";
                        ++n_products;
                    }
                }
            }
        }
    }
    std::snprintf( buf, sizeof( buf ),
                   "\"fn\":\"item::burn\",\"destroyed\":%s,\"weight_g\":%lld,\"fuel_produced\":%.9g,"
                   "\"smoke_produced\":%.9g,\"n_products\":%d,\"products\":[",
                   destroyed ? "true" : "false", static_cast<long long>( units::to_gram( new_weight ) ),
                   static_cast<double>( frd.fuel_produced ), static_cast<double>( frd.smoke_produced ), n_products );
    draw_recorder::mark( "ret", buf + products + "]" );
    return destroyed;
}

// Creature side (creature cases). Strings ride in the marks as JSON strings.
// Each call mark carries the caller's pc (like draws), so a replay can tell a
// call Fields makes from one made inside another call it cannot bracket
// (BN's virtual deal_damage).
extern "C" char __executable_start;
std::string call_pc( void *ret_addr )
{
    char buf[48];
    std::snprintf( buf, sizeof( buf ), ",\"pc\":\"0x%" PRIxPTR "\"",
                   reinterpret_cast<uintptr_t>( ret_addr ) - reinterpret_cast<uintptr_t>( &__executable_start ) );
    return buf;
}
using efftype_ref = const string_id<effect_type> &;
using bp_ref = const string_id<body_part_type> &;

extern "C" void __real__ZN8Creature10add_effectERK6effectbb( Creature *, const effect &, bool, bool );
extern "C" void __wrap__ZN8Creature10add_effectERK6effectbb( Creature *self, const effect &e, bool force,
        bool deferred )
{
    draw_recorder::mark( "call", "\"fn\":\"Creature::add_effect\",\"eff\":" + quote( e.get_id().str() ) +
                         ",\"dur\":" + std::to_string( to_turns<int>( e.get_duration() ) ) + ",\"intensity\":" +
                         std::to_string( e.get_intensity() )  + call_pc( __builtin_return_address( 0 ) ) );
    __real__ZN8Creature10add_effectERK6effectbb( self, e, force, deferred );
    draw_recorder::mark( "ret", "\"fn\":\"Creature::add_effect\"" );
}

extern "C" void
__real__ZN8Creature10add_effectERK9string_idI11effect_typeERK13time_durationRKS0_I14body_part_typeEibb(
    Creature *, efftype_ref, const time_duration &, bp_ref, int, bool, bool );
extern "C" void
__wrap__ZN8Creature10add_effectERK9string_idI11effect_typeERK13time_durationRKS0_I14body_part_typeEibb(
    Creature *self, efftype_ref id, const time_duration &dur, bp_ref bp, int intensity, bool force,
    bool deferred )
{
    draw_recorder::mark( "call", "\"fn\":\"Creature::add_effect\",\"eff\":" + quote( id.str() ) +
                         ",\"dur\":" + std::to_string( to_turns<int>( dur ) ) + ",\"intensity\":" +
                         std::to_string( intensity )  + call_pc( __builtin_return_address( 0 ) ) );
    __real__ZN8Creature10add_effectERK9string_idI11effect_typeERK13time_durationRKS0_I14body_part_typeEibb(
        self, id, dur, bp, intensity, force, deferred );
    draw_recorder::mark( "ret", "\"fn\":\"Creature::add_effect\"" );
}

extern "C" void __real__ZN8Creature16check_dead_stateEv( Creature * );
extern "C" void __wrap__ZN8Creature16check_dead_stateEv( Creature *self )
{
    draw_recorder::mark( "call", "\"fn\":\"Creature::check_dead_state\""  + call_pc( __builtin_return_address( 0 ) ) );
    __real__ZN8Creature16check_dead_stateEv( self );
    draw_recorder::mark( "ret", "\"fn\":\"Creature::check_dead_state\"" );
}

extern "C" bool __real__ZN8Creature14add_env_effectERK9string_idI11effect_typeERKS0_I14body_part_typeEiRK13time_duration(
    Creature *, efftype_ref, bp_ref, int, const time_duration & );
extern "C" bool __wrap__ZN8Creature14add_env_effectERK9string_idI11effect_typeERKS0_I14body_part_typeEiRK13time_duration(
    Creature *self, efftype_ref id, bp_ref vec, int strength, const time_duration &dur )
{
    draw_recorder::mark( "call", "\"fn\":\"Creature::add_env_effect\",\"eff\":" + quote( id.str() ) +
                         ",\"vector\":" + quote( vec.str() ) + ",\"strength\":" + std::to_string( strength ) +
                         ",\"dur\":" + std::to_string( to_turns<int>( dur ) )  + call_pc( __builtin_return_address( 0 ) ) );
    const bool r =
        __real__ZN8Creature14add_env_effectERK9string_idI11effect_typeERKS0_I14body_part_typeEiRK13time_duration(
            self, id, vec, strength, dur );
    draw_recorder::mark( "ret", std::string( "\"fn\":\"Creature::add_env_effect\",\"r\":" ) +
                         ( r ? "true" : "false" ) );
    return r;
}

extern "C" bool __real__ZN7monster11make_fungusEv( monster * );
extern "C" bool __wrap__ZN7monster11make_fungusEv( monster *self )
{
    draw_recorder::mark( "call", "\"fn\":\"monster::make_fungus\""  + call_pc( __builtin_return_address( 0 ) ) );
    const bool r = __real__ZN7monster11make_fungusEv( self );
    draw_recorder::mark( "ret", std::string( "\"fn\":\"monster::make_fungus\",\"r\":" ) +
                         ( r ? "true" : "false" ) );
    return r;
}

extern "C" bool __real__ZN8teleport8teleportER8Creatureiibb( Creature &, int, int, bool, bool );
extern "C" bool __wrap__ZN8teleport8teleportER8Creatureiibb( Creature &c, int lo, int hi, bool safe,
        bool add_teleglow )
{
    draw_recorder::mark( "call", "\"fn\":\"teleport::teleport\""  + call_pc( __builtin_return_address( 0 ) ) );
    const bool r = __real__ZN8teleport8teleportER8Creatureiibb( c, lo, hi, safe, add_teleglow );
    // Where it landed, in layout tile coordinates (creature cases start at BUB0).
    const tripoint_bub_ms at = c.bub_pos();
    draw_recorder::mark( "ret", std::string( "\"fn\":\"teleport::teleport\",\"r\":" ) +
                         ( r ? "true" : "false" ) + ",\"x\":" + std::to_string( at.x() - BUB0.x() ) +
                         ",\"y\":" + std::to_string( at.y() - BUB0.y() ) );
    return r;
}

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
