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
    # One addr2line process for all pcs (loading the debug info dominates).
    # -i: inline helpers from rng.h (roll_remainder(float), random_entry*)
    # would otherwise hide their caller; -a marks where each pc's frames start.
    out = subprocess.run(
        ["addr2line", "-a", "-i", "-C", "-f", "-e", binary] + [hex(int(p, 16) - 1) for p in pcs],
        check=True, capture_output=True, text=True).stdout.splitlines()
    groups, cur = [], None
    for line in out:
        if line.startswith("0x") and (cur is None or len(cur) % 2 == 0):
            cur = []
            groups.append(cur)
        else:
            cur.append(line)
    assert len(groups) == len(pcs), (len(groups), len(pcs))

    def clean(fn, loc):
        loc = loc.split(" (discriminator")[0]
        if "/src/" in loc:
            loc = "src/" + loc.split("/src/", 1)[1].replace("sol/../", "")
        return f"{loc} ({fn.split('(')[0]})"

    res = {}
    for p, g in zip(pcs, groups):
        frames = [clean(g[i], g[i + 1]) for i in range(0, len(g), 2)]
        # The caller is the innermost frame that is game code: not rng.h's
        # inline helpers, not libstdc++ (std::ranges::for_each lambdas inline
        # the rule into the loop over the field cache).
        game = [f for f in frames if not f.startswith("src/rng.h") and "/include/c++/" not in f]
        pick = game[0] if game else frames[0]
        res[p] = pick + ("" if pick == frames[0] else " via " + frames[0].split(" (")[0])
    return res


def main():
    binary, case = sys.argv[1], sys.argv[2]
    summary = "--summary" in sys.argv[3:]
    objs = [json.loads(l) for l in open(case)]
    # depth-0 draws, and `call` marks of wrapped calls (session 3) that carry a pc
    def wants(o):
        return (o["k"] == "draw" and o.get("d") == 0) or (o["k"] == "mark" and "pc" in o)
    pcs = {o["pc"] for o in objs if wants(o)}
    where = resolve(binary, pcs)
    if summary:
        c = Counter((where[o["pc"]], o["fn"]) for o in objs
                    if o["k"] == "draw" and o.get("d") == 0)
        for (loc, fn), n in sorted(c.items(), key=lambda kv: -kv[1]):
            print(f"{n:8d}  {fn:16s} {loc}")
        return
    for o in objs:
        if wants(o):
            o["src"] = where[o["pc"]]
        print(json.dumps(o, separators=(",", ":")))


if __name__ == "__main__":
    main()
