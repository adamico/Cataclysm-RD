#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || ! -d "$1/src" ]]; then
  echo "usage: $0 BN_ROOT  (a Baseline checkout with src/)" >&2
  exit 2
fi

ctags -R -x --languages=C++ --map-C++=+.h --kinds-C++=cfgmnpstuev \
  --_xformat='%K %N' "$1/src" |
  awk '
    $1 ~ /^(class|struct|namespace|enum|union|typedef)$/ { print "qualifier " $2 }
    $2 ~ /_/ && $2 !~ /^_/ { print "name " $2 }
  ' |
  LC_ALL=C sort -u
