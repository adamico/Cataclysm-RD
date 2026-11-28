#!/usr/bin/env bash
# Fields spike (issue #14) — Fields-only soak run, C++ side (skeleton).
#
#   spike/soak/run_soak.sh <fields_recorder> <bn-src-dir> <out-dir> [layout] [turns] [seeds-file] [--record]
#
# Seeds the layout, runs <turns> turns for every seed in <seeds-file>, and writes
# <out-dir>/seed-<s>.dump.jsonl per seed (plus .case.jsonl with --record), then a
# per-seed summary table <out-dir>/summary.tsv. The Rust side (session 3) must
# produce the same dumps from the same layout/seeds/turns; comparison is
# statistical over the seed set (see DUMP_FORMAT.md for run conventions).
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
REC=$(realpath "$1"); BN=$(realpath "$2"); OUT=$(realpath -m "$3")
LAYOUT=$(realpath "${4:-$HERE/layouts/smoke.layout}")
TURNS=${5:-100}
SEEDS_FILE=$(realpath "${6:-$HERE/seeds.txt}")
EXTRA=${7:-}
SEEDS=$(grep -v '^#' "$SEEDS_FILE" | tr -s ' \n' ',' | sed 's/^,//; s/,$//')
mkdir -p "$OUT"
"$REC" run --bn "$BN" --user-dir "$OUT/.userdir" --layout "$LAYOUT" \
    --seeds "$SEEDS" --turns "$TURNS" --out-dir "$OUT" $EXTRA
rm -rf "$OUT/.userdir"
python3 -I "$HERE/summarise_dumps.py" "$OUT"/seed-*.dump.jsonl > "$OUT/summary.tsv"
echo "summary: $OUT/summary.tsv"
