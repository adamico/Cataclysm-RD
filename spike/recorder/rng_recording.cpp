// Fields spike (issue #14) — recording copy of BN src/rng.cpp (Baseline e7b4976).
//
// Every function body below is BN's, unchanged in behaviour; the only
// additions are the `REC_*` hooks that log each helper-level draw to the case
// file opened by draw_recorder::begin(). `diff` this file against BN's
// src/rng.cpp to see the hooks: nothing else differs in the draw logic.
//
// The recorder program links this object INSTEAD of BN's rng.cpp.o.
// Case file format: spike/recorder/CASE_FORMAT.md.

#include "rng.h"
#include "weighted_list.h"

#include <cmath>
#include <chrono>
#include <utility>

#include "calendar.h"
#include "cata_utility.h"
#include "units.h"

// ---- recording machinery (spike addition) ----------------------------------
#include "draw_recorder.h"

#include <cinttypes>
#include <cstdio>
#include <string>

extern "C" char __executable_start; // NOLINT: linker-provided, base of the image

static cata_default_random_engine &main_rng_engine();
static bool tl_is_worker_fwd();

namespace
{
struct rec_state {
    std::FILE *out = nullptr;
    bool paused = false;
    int depth = 0;
    uint64_t seq = 0;          // helper calls recorded (all depths)
    uint64_t top = 0;          // depth-0 helper calls recorded
    uint64_t bypass_steps = 0; // raw engine steps taken outside any helper
    uint64_t unrecorded = 0;   // helper calls while paused / on worker threads
    cata_default_random_engine expected; // engine state after the last helper
    bool have_expected = false;
};
rec_state R;

std::string fmt_d( double v )
{
    char buf[40];
    std::snprintf( buf, sizeof( buf ), "%.17g", v );
    std::string s( buf );
    if( s == "inf" || s == "-inf" || s == "nan" || s == "-nan" ) {
        return "\"" + s + "\"";
    }
    return s;
}
std::string fmt_i( long long v )
{
    return std::to_string( v );
}
std::string fmt_u( unsigned long long v )
{
    return std::to_string( v );
}
std::string fmt_b( bool v )
{
    return v ? "true" : "false";
}

bool recording()
{
    return R.out != nullptr && !R.paused && !tl_is_worker_fwd();
}

// Count raw engine steps from `from` to `to` (bounded). -1 if not found.
long long steps_between( cata_default_random_engine from, const cata_default_random_engine &to )
{
    for( long long n = 0; n <= 10000000; ++n ) {
        if( from == to ) {
            return n;
        }
        from();
    }
    return -1;
}

// Called at entry of every public helper.
struct rec_scope {
    bool on;
    uint64_t index = 0;
    uintptr_t pc = 0;
    explicit rec_scope( void *ret_addr ) : on( false ) {
        if( !recording() ) {
            if( R.out != nullptr && R.depth == 0 ) {
                ++R.unrecorded;
            }
            return;
        }
        on = true;
        if( R.depth == 0 ) {
            // Bypass detection: did the engine move since the last helper returned?
            if( R.have_expected && !( R.expected == main_rng_engine() ) ) {
                const long long n = steps_between( R.expected, main_rng_engine() );
                std::fprintf( R.out,
                              "{\"k\":\"bypass\",\"before\":%" PRIu64 ",\"steps\":%lld}\n",
                              R.seq, n );
                if( n > 0 ) {
                    R.bypass_steps += static_cast<uint64_t>( n );
                }
            }
            pc = reinterpret_cast<uintptr_t>( ret_addr ) -
                 reinterpret_cast<uintptr_t>( &__executable_start );
        }
        index = R.seq++;
        ++R.depth;
    }
    // Write the line. `args` and `result` are already JSON.
    void done( const char *fn, const std::string &args, const std::string &result ) {
        if( !on ) {
            return;
        }
        --R.depth;
        if( R.depth == 0 ) {
            ++R.top;
            std::fprintf( R.out,
                          "{\"k\":\"draw\",\"i\":%" PRIu64 ",\"d\":0,\"fn\":\"%s\",\"args\":[%s],"
                          "\"r\":%s,\"pc\":\"0x%" PRIxPTR "\"}\n",
                          index, fn, args.c_str(), result.c_str(), pc );
            R.expected = main_rng_engine();
            R.have_expected = true;
        } else {
            std::fprintf( R.out,
                          "{\"k\":\"draw\",\"i\":%" PRIu64 ",\"d\":%d,\"fn\":\"%s\",\"args\":[%s],"
                          "\"r\":%s}\n",
                          index, R.depth, fn, args.c_str(), result.c_str() );
        }
    }
};
} // namespace

#define REC_BEGIN rec_scope rec_( __builtin_return_address( 0 ) )

namespace draw_recorder
{
void begin( const std::string &path, const std::string &header_json )
{
    if( R.out != nullptr ) {
        end();
    }
    R = rec_state{};
    R.out = std::fopen( path.c_str(), "w" );
    if( R.out == nullptr ) {
        std::fprintf( stderr, "draw_recorder: cannot open %s\n", path.c_str() );
        return;
    }
    std::fprintf( R.out,
                  "{\"k\":\"header\",\"format\":\"bn-draws/1\",\"baseline\":\"e7b4976\"%s%s}\n",
                  header_json.empty() ? "" : ",", header_json.c_str() );
    R.expected = main_rng_engine();
    R.have_expected = true;
}
void mark( const std::string &label, const std::string &extra_json )
{
    if( R.out == nullptr ) {
        return;
    }
    std::fprintf( R.out, "{\"k\":\"mark\",\"before\":%" PRIu64 ",\"label\":\"%s\"%s%s}\n", R.seq,
                  label.c_str(), extra_json.empty() ? "" : ",", extra_json.c_str() );
}
void pause()
{
    R.paused = true;
}
void resume()
{
    R.paused = false;
    // Draws made while paused are not bypasses.
    R.expected = main_rng_engine();
    R.have_expected = true;
}
void end()
{
    if( R.out == nullptr ) {
        return;
    }
    if( R.have_expected && !R.paused && !( R.expected == main_rng_engine() ) ) {
        const long long n = steps_between( R.expected, main_rng_engine() );
        std::fprintf( R.out, "{\"k\":\"bypass\",\"before\":%" PRIu64 ",\"steps\":%lld}\n", R.seq,
                      n );
        if( n > 0 ) {
            R.bypass_steps += static_cast<uint64_t>( n );
        }
    }
    std::fprintf( R.out,
                  "{\"k\":\"footer\",\"draws\":%" PRIu64 ",\"top\":%" PRIu64
                  ",\"bypass_steps\":%" PRIu64 ",\"unrecorded\":%" PRIu64 "}\n",
                  R.seq, R.top, R.bypass_steps, R.unrecorded );
    std::fclose( R.out );
    R.out = nullptr;
}
bool active()
{
    return R.out != nullptr;
}
} // namespace draw_recorder
// ---- end recording machinery -----------------------------------------------

unsigned int rng_bits()
{
    REC_BEGIN;
    // Whole uint range.
    static std::uniform_int_distribution<unsigned int> rng_uint_dist;
    const unsigned int r = rng_uint_dist( rng_get_engine() );
    rec_.done( "rng_bits", "", fmt_u( r ) );
    return r;
}

int rng( int lo, int hi )
{
    REC_BEGIN;
    const int lo0 = lo;
    const int hi0 = hi;
    static std::uniform_int_distribution<int> rng_int_dist;
    if( lo > hi ) {
        std::swap( lo, hi );
    }
    const int r = rng_int_dist( rng_get_engine(), std::uniform_int_distribution<>::param_type( lo,
                                hi ) );
    rec_.done( "rng", fmt_i( lo0 ) + "," + fmt_i( hi0 ), fmt_i( r ) );
    return r;
}

double rng_float( double lo, double hi )
{
    REC_BEGIN;
    const double lo0 = lo;
    const double hi0 = hi;
    static std::uniform_real_distribution<double> rng_real_dist;
    if( lo > hi ) {
        std::swap( lo, hi );
    }
    const double r = rng_real_dist( rng_get_engine(),
                                    std::uniform_real_distribution<>::param_type( lo, hi ) );
    rec_.done( "rng_float", fmt_d( lo0 ) + "," + fmt_d( hi0 ), fmt_d( r ) );
    return r;
}

units::angle random_direction()
{
    REC_BEGIN;
    const units::angle r = rng_float( 0_pi_radians, 2_pi_radians );
    rec_.done( "random_direction", "", fmt_d( units::to_radians( r ) ) );
    return r;
}

double normal_roll( double mean, double stddev )
{
    REC_BEGIN;
    static std::normal_distribution<double> rng_normal_dist;
    const double r = rng_normal_dist( rng_get_engine(),
                                      std::normal_distribution<>::param_type( mean, stddev ) );
    rec_.done( "normal_roll", fmt_d( mean ) + "," + fmt_d( stddev ), fmt_d( r ) );
    return r;
}

double exponential_roll( double lambda )
{
    REC_BEGIN;
    static std::exponential_distribution<double> rng_exponential_dist;
    const double r = rng_exponential_dist( rng_get_engine(),
                                           std::exponential_distribution<>::param_type( lambda ) );
    rec_.done( "exponential_roll", fmt_d( lambda ), fmt_d( r ) );
    return r;
}

double rng_exponential( double min, double mean )
{
    REC_BEGIN;
    const double adjusted_mean = mean - min;
    double r;
    if( adjusted_mean <= 0.0 ) {
        r = 0.0;
    } else {
        // lambda = 1 / mean
        r = min + exponential_roll( 1.0 / adjusted_mean );
    }
    rec_.done( "rng_exponential", fmt_d( min ) + "," + fmt_d( mean ), fmt_d( r ) );
    return r;
}

bool one_in( int chance )
{
    REC_BEGIN;
    const bool r = ( chance <= 1 || rng( 0, chance - 1 ) == 0 );
    rec_.done( "one_in", fmt_i( chance ), fmt_b( r ) );
    return r;
}

bool one_turn_in( const time_duration &duration )
{
    REC_BEGIN;
    const int turns = to_turns<int>( duration );
    const bool r = one_in( turns );
    rec_.done( "one_turn_in", fmt_i( turns ), fmt_b( r ) );
    return r;
}

bool x_in_y( double x, double y )
{
    REC_BEGIN;
    const bool r = rng_float( 0.0, 1.0 ) <= x / y;
    rec_.done( "x_in_y", fmt_d( x ) + "," + fmt_d( y ), fmt_b( r ) );
    return r;
}

bool check( units::probability p )
{
    REC_BEGIN;
    const auto pm = units::to_one_in_million( p );
    const bool r = rng( 0, 1000000 - 1 ) < pm;
    rec_.done( "check", fmt_i( pm ), fmt_b( r ) );
    return r;
}

int dice( int number, int sides )
{
    REC_BEGIN;
    int ret = 0;
    for( int i = 0; i < number; i++ ) {
        ret += rng( 1, sides );
    }
    rec_.done( "dice", fmt_i( number ) + "," + fmt_i( sides ), fmt_i( ret ) );
    return ret;
}

// probabilistically round a double to an int
// 1.3 has a 70% chance of rounding to 1, 30% chance to 2.
int roll_remainder( double value )
{
    REC_BEGIN;
    double integ;
    double frac = modf( value, &integ );
    if( value > 0.0 && value > integ && x_in_y( frac, 1.0 ) ) {
        integ++;
    } else if( value < 0.0 && value < integ && x_in_y( -frac, 1.0 ) ) {
        integ--;
    }

    rec_.done( "roll_remainder", fmt_d( value ), fmt_i( static_cast<int>( integ ) ) );
    return integ;
}

// http://www.cse.yorku.ca/~oz/hash.html
// for world seeding.
int djb2_hash( const unsigned char *input )
{
    unsigned int hash = 5381;
    unsigned char c = *input++;
    while( c != '\0' ) {
        hash = ( ( hash << 5 ) + hash ) + c; /* hash * 33 + c */
        c = *input++;
    }
    return hash;
}

double rng_normal( double lo, double hi )
{
    REC_BEGIN;
    const double lo0 = lo;
    const double hi0 = hi;
    if( lo > hi ) {
        std::swap( lo, hi );
    }

    const double stddev = ( hi - lo ) / 4;
    double r;
    if( stddev == 0.0 ) {
        r = hi;
    } else {
        double val = normal_roll( ( hi + lo ) / 2, stddev );
        r = clamp( val, lo, hi );
    }
    rec_.done( "rng_normal", fmt_d( lo0 ) + "," + fmt_d( hi0 ), fmt_d( r ) );
    return r;
}

// Main-thread global engine (default, time-seeded).
static cata_default_random_engine &main_rng_engine()
{
    // NOLINTNEXTLINE(cata-determinism)
    static cata_default_random_engine eng(
        std::chrono::high_resolution_clock::now().time_since_epoch().count() );
    return eng;
}

// Per-worker-thread engine.  Inactive (flag=false) on the main thread.
// NOLINTNEXTLINE(cata-determinism)
static thread_local bool tl_is_worker = false;
// NOLINTNEXTLINE(cata-determinism)
static thread_local cata_default_random_engine tl_worker_engine;

static bool tl_is_worker_fwd()
{
    return tl_is_worker;
}

cata_default_random_engine &rng_get_engine()
{
    if( tl_is_worker ) {
        return tl_worker_engine;
    }
    return main_rng_engine();
}

void rng_set_engine_seed( unsigned int seed )
{
    if( seed != 0 ) {
        main_rng_engine().seed( seed );
    }
    if( R.out != nullptr ) {
        std::fprintf( R.out, "{\"k\":\"seed\",\"before\":%" PRIu64 ",\"seed\":%u}\n", R.seq, seed );
        R.expected = main_rng_engine();
        R.have_expected = true;
    }
}

void rng_set_worker_seed( unsigned int seed )
{
    tl_is_worker = true;
    tl_worker_engine.seed( seed );
}

namespace weighted_list_detail
{
unsigned int gen_rand_i()
{
    return rng_bits();
}
} // namespace weighted_list_detail
