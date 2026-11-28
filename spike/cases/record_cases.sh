#!/usr/bin/env bash
# Fields spike (issue #14) session 3 — record the per-rule replay cases.
#
#   spike/cases/record_cases.sh <fields_recorder> <bn-src-dir> [layout-name ...]
#
# For every layout in spike/cases/layouts (or the ones named), runs the
# recorder over seeds 1..8 for 5 turns with --record --dump-every, and writes
# spike/cases/recorded/<layout>/seed-<s>.{case,dump}.jsonl ($OUT_ROOT overrides
# the root; $SEEDS and $TURNS override the run). Layouts run in
# parallel (each process loads BN Content, ~13 s).
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
REC=$(realpath "$1"); BN=$(realpath "$2"); shift 2
SEEDS=${SEEDS:-1,2,3,4,5,6,7,8}
TURNS=${TURNS:-5}
if [[ $# -eq 0 ]]; then
    set -- $(cd "$HERE/layouts" && ls *.layout | sed 's/\.layout$//')
fi
SCRATCH=${SCRATCH:-$(mktemp -d)}; mkdir -p "$SCRATCH"
run_one() {
    local name=$1 out="${OUT_ROOT:-$HERE/recorded}/$1"
    rm -rf "$out"; mkdir -p "$out"
    "$REC" run --bn "$BN" --user-dir "$SCRATCH/ud-$name" --layout "$HERE/layouts/$name.layout" \
        --seeds "$SEEDS" --turns "$TURNS" --out-dir "$out" --record --dump-every --case "$name" \
        > "$SCRATCH/$name.log" 2>&1 || { echo "FAILED $name (see $SCRATCH/$name.log)"; return 1; }
    rm -rf "$SCRATCH/ud-$name"
    # pcs are only valid for this binary: resolve them to BN source lines now.
    for c in "$out"/*.case.jsonl; do
        python3 -I "$HERE/../recorder/annotate_case.py" "$REC" "$c" > "$c.tmp" && mv "$c.tmp" "$c"
    done
    echo "recorded $name"
}
export -f run_one; export HERE REC BN SEEDS TURNS SCRATCH OUT_ROOT
printf '%s\n' "$@" | xargs -P 8 -I{} bash -c 'run_one {}'
