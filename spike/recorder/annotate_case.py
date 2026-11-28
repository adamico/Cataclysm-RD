"""Resolve the `pc` of every depth-0 draw in a case file to a BN source line.

    python3 -I spike/recorder/annotate_case.py <fields_recorder binary> <case.jsonl> [--summary]

Default: re-emits the case with a `src` field ("file:line (function)") on depth-0
draws. --summary: prints draw counts per source line instead, so you can see
which rules (and which non-Fields code, e.g. item burning) draw.
Needs binutils' addr2line; the Reference build is RelWithDebInfo.
"""
import json
import subprocess
import sys
from collections import Counter


def resolve(binary, pcs):
    pcs = sorted(pcs)
    if not pcs:
        return {}
    # addr2line wants the call instruction, not the return address: pc - 1.
    res = {}
    for p in pcs:
        lines = subprocess.run(
            ["addr2line", "-C", "-f", "-e", binary, hex(int(p, 16) - 1)],
            check=True, capture_output=True, text=True).stdout.splitlines()
        fn, loc = lines[0], lines[1]
        loc = loc.split(" (discriminator")[0]
        if "/src/" in loc:
            loc = "src/" + loc.split("/src/", 1)[1]
        res[p] = f"{loc} ({fn.split('(')[0]})"
    return res


def main():
    binary, case = sys.argv[1], sys.argv[2]
    summary = "--summary" in sys.argv[3:]
    objs = [json.loads(l) for l in open(case)]
    pcs = {o["pc"] for o in objs if o["k"] == "draw" and o.get("d") == 0}
    where = resolve(binary, pcs)
    if summary:
        c = Counter((where[o["pc"]], o["fn"]) for o in objs
                    if o["k"] == "draw" and o.get("d") == 0)
        for (loc, fn), n in sorted(c.items(), key=lambda kv: -kv[1]):
            print(f"{n:8d}  {fn:16s} {loc}")
        return
    for o in objs:
        if o["k"] == "draw" and o.get("d") == 0:
            o["src"] = where[o["pc"]]
        print(json.dumps(o, separators=(",", ":")))


if __name__ == "__main__":
    main()
