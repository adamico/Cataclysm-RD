#!/usr/bin/env bash
# Fields spike (issue #14) — build the recorder against an existing Reference build.
#
#   spike/recorder/build.sh <bn-src-dir> <bn-build-dir> <out-dir>
#
# It reuses BN's own compile and link commands, so the recorder is compiled with
# exactly the Reference build's flags:
#   * rng_recording.cpp  is compiled with the compile command of BN's src/rng.cpp
#   * fields_recorder.cpp is compiled with the compile command of BN's src/main.cpp
#   * the link command of the `cataclysm-bn` target is replayed with
#     rng.cpp.o -> rng_recording.o and main.cpp.o -> fields_recorder.o.
# No BN source or object is modified.
set -euo pipefail

BN_SRC=$(realpath "$1")
BN_BUILD=$(realpath "$2")
OUT=$(realpath -m "$3")
HERE=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$OUT"

# Compile command for a BN source file, from compile_commands.json.
compile_cmd() {
    python3 -I -c '
import json, sys
src = sys.argv[2]
for e in json.load(open(sys.argv[1])):
    if e["file"] == src:
        print(e["command"]); break
else:
    sys.exit("no compile command for " + src)
' "$BN_BUILD/compile_commands.json" "$1"
}

# Rewrite "-o <obj> -c <src>" of a BN compile command onto our own files.
retarget() { # cmd old_obj old_src new_obj new_src
    local cmd=$1
    cmd=${cmd/"-o $2"/"-o $4"}
    cmd=${cmd/"-c $3"/"-c $5"}
    printf '%s' "$cmd"
}

cd "$BN_BUILD"

RNG_CMD=$(compile_cmd "$BN_SRC/src/rng.cpp")
RNG_CMD=$(retarget "$RNG_CMD" "src/CMakeFiles/cataclysm-bn-common.dir/rng.cpp.o" "$BN_SRC/src/rng.cpp" \
    "$OUT/rng_recording.o" "$HERE/rng_recording.cpp")
echo "[recorder] compile rng_recording.cpp"
eval "$RNG_CMD -I$HERE"

MAIN_CMD=$(compile_cmd "$BN_SRC/src/main.cpp")
MAIN_CMD=$(retarget "$MAIN_CMD" "src/CMakeFiles/cataclysm-bn.dir/main.cpp.o" "$BN_SRC/src/main.cpp" \
    "$OUT/fields_recorder.o" "$HERE/fields_recorder.cpp")
echo "[recorder] compile fields_recorder.cpp"
eval "$MAIN_CMD -I$HERE"

LINK_CMD=$(ninja -t commands cataclysm-bn | tail -n 1)
for need in "src/CMakeFiles/cataclysm-bn-common.dir/rng.cpp.o" "src/CMakeFiles/cataclysm-bn.dir/main.cpp.o" "-o src/cataclysm-bn"; do
    [[ $LINK_CMD == *"$need"* ]] || { echo "link command lacks '$need'" >&2; exit 1; }
done
LINK_CMD=${LINK_CMD/"src/CMakeFiles/cataclysm-bn-common.dir/rng.cpp.o"/"$OUT/rng_recording.o"}
LINK_CMD=${LINK_CMD/"src/CMakeFiles/cataclysm-bn.dir/main.cpp.o"/"$OUT/fields_recorder.o"}
LINK_CMD=${LINK_CMD/"-o src/cataclysm-bn"/"-o $OUT/fields_recorder"}
# do not overwrite the Reference build's own link dependency file
LINK_CMD=${LINK_CMD/"-Xlinker --dependency-file=src/CMakeFiles/cataclysm-bn.dir/link.d"/}
echo "[recorder] link fields_recorder"
eval "$LINK_CMD"

# Proof that BN's rng.cpp did not sneak in: its object must not be on the link line,
# and the recorder's marker symbol must be present.
if [[ $LINK_CMD == *"dir/rng.cpp.o"* ]]; then echo "BN rng.cpp.o still linked" >&2; exit 1; fi
nm -C "$OUT/fields_recorder" | grep "draw_recorder::begin" >/dev/null || { echo "recorder symbols missing" >&2; exit 1; }
echo "[recorder] built $OUT/fields_recorder"
