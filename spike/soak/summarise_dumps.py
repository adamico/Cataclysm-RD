"""Summarise Observable dumps (bn-fields-dump/1): one TSV row per (seed, turn, field type).

Columns: seed, turn, type, tiles, sum_intensity, max_intensity, mean_age.
Reads the last-written snapshot per (seed, turn). Used by the soak runner; the
statistical C++-vs-Rust comparison is built on these rows in session 3.
"""
import json
import sys
from collections import defaultdict


def snapshots(path):
    head, rows = None, []
    with open(path) as f:
        for line in f:
            o = json.loads(line)
            if o["k"] == "dump":
                if head is not None:
                    yield head, rows
                head, rows = o, []
            elif o["k"] == "f":
                rows.append(o)
    if head is not None:
        yield head, rows


def main(paths):
    print("seed\tturn\ttype\ttiles\tsum_intensity\tmax_intensity\tmean_age")
    for p in paths:
        for head, rows in snapshots(p):
            assert head["format"] == "bn-fields-dump/1", head
            assert head["entries"] == len(rows), (p, head)
            agg = defaultdict(list)
            for r in rows:
                if r["alive"]:
                    agg[r["t"]].append(r)
            for t in sorted(agg):
                rs = agg[t]
                print(f'{head["seed"]}\t{head["turn"]}\t{t}\t{len(rs)}\t'
                      f'{sum(r["n"] for r in rs)}\t{max(r["n"] for r in rs)}\t'
                      f'{sum(r["age"] for r in rs) / len(rs):.2f}')


if __name__ == "__main__":
    main(sys.argv[1:])
