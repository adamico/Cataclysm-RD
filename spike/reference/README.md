# Reference build (headless BN, Baseline e7b4976)

The Reference build is BN v0.13.0 at the Baseline commit `e7b4976`, built
untouched as a curses (no tiles, no sound, no tests) binary. Its objects are
what the recorder links against (`spike/recorder/`).

## Prerequisites (as used on 2026-11-28)

- clang/clang++ (system: clang with GCC 16.2.1 libstdc++), cmake, ninja
- ncursesw, sqlite3, zlib dev packages (Lua is vendored in `src/lua`)
- no ccache, no mold (the defaults in BN's `linux-curses` preset need both,
  so the preset is not used)

## Commands

```bash
# 1. Sources: a shallow clone at the Baseline commit, never modified.
BN=/path/to/bn          # outside this repo
git init "$BN" && git -C "$BN" remote add origin https://github.com/cataclysmbn/Cataclysm-BN.git
git -C "$BN" fetch --depth 1 origin e7b497637d452928ef1a9a77f175c5fd9c3fa75e && git -C "$BN" checkout FETCH_HEAD
BUILD=/path/to/bn-build # out-of-tree, anywhere outside $BN

# 2. Configure (~25 s)
cmake -S "$BN" -B "$BUILD" -G Ninja \
  -DCMAKE_BUILD_TYPE=RelWithDebInfo \
  -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ \
  -DCURSES=ON -DTILES=OFF -DSOUND=OFF -DTESTS=OFF \
  -DUSE_PREFIX_DATA_DIR=OFF \
  -DBACKTRACE=OFF -DCATA_CCACHE=OFF -DCATA_FORMAT_TARGETS=OFF \
  -DLUA_DOCS_ON_BUILD=OFF -DCMAKE_EXPORT_COMPILE_COMMANDS=ON

# 3. Build the curses game binary (all BN objects)
ninja -C "$BUILD" -j22 cataclysm-bn
```

`USE_PREFIX_DATA_DIR=OFF` is required: it defaults to ON on Linux and bakes an
install-prefix data path (`share/cataclysm-bn/`) into every object through a
global `-DPREFIX`, so the binary cannot find `data/` in the source tree.
BN's own `linux-curses` preset sets it OFF. Changing it later forces a full
rebuild.

`CMAKE_EXPORT_COMPILE_COMMANDS=ON` is required: the recorder's build script
reads `compile_commands.json` to compile its two files with exactly BN's flags.

The result is `$BUILD/src/cataclysm-bn` plus the object library
`$BUILD/src/CMakeFiles/cataclysm-bn-common.dir/**.o`. BN data is read from the
source tree (`$BN/data`), so the build directory holds no Content.

## Recorder

```bash
spike/recorder/build.sh "$BN" "$BUILD" "$OUT"   # -> $OUT/fields_recorder
```

See `spike/recorder/README.md`.

## Measured

See `spike/spike-log.md` for wall-clock of each step on this machine
(24 threads, 61 GB RAM).
