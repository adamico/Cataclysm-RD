"""Draw-site coverage of the recorded cases (issue #14, session 3).

    python3 -I spike/cases/coverage.py <bn-src-dir> [case.jsonl ...]

Lists every line of BN's src/map/map_field.cpp that calls a dice helper, with
the number of recorded draws made from it over all cases in
spike/cases/recorded (or the cases given), plus the draws made outside the
Fields files (item::burn and the like). A draw site with 0 draws is a gap.
"""
import glob
import json
import os
import re
import sys
from collections import Counter

HELPERS = r"\b(rng|rng_float|one_in|x_in_y|dice|roll_remainder|random_entry\w*|one_turn_in|rng_normal|normal_roll)\s*\("


def rule_of(lines, n):
    """Nearest preceding rule header (BN's '// ---- fd_x' comments or a function)."""
    for i in range(n - 1, -1, -1):
        m = re.match(r"\s*// ---- (.+?) -*$", lines[i]) or re.match(r"^(?:auto |void |bool |int )?(?:map::)?(\w+)\(.*\)", lines[i])
        if m:
            return m.group(1).strip()
    return "?"


def main():
    bn = sys.argv[1]
    paths = sys.argv[2:] or glob.glob(os.path.join(os.path.dirname(__file__), "recorded", "*", "*.case.jsonl"))
    src = open(os.path.join(bn, "src/map/map_field.cpp")).read().splitlines()
    sites = {}
    for n, line in enumerate(src, 1):
        code = line.split("//")[0]
        if re.search(HELPERS, code):
            sites[n] = (rule_of(src, n), code.strip())
    hits, foreign = Counter(), Counter()
    for p in paths:
        for l in open(p):
            o = json.loads(l)
            if o["k"] == "draw" and o.get("d") == 0:
                s = o.get("src", "")
                m = re.match(r"src/map/map_field\.cpp:(\d+)", s)
                if m:
                    hits[int(m.group(1))] += 1
                else:
                    foreign[s.split(" (")[0]] += 1
    print(f"| line | rule | call | draws |\n|---|---|---|---|")
    for n, (rule, code) in sites.items():
        print(f"| {n} | {rule} | `{code[:70].replace('|', '/')}` | {hits.get(n, 0)} |")
    unknown = sorted(set(hits) - set(sites))
    if unknown:
        print(f"\nDraws attributed to lines with no helper call on the line (multi-line calls): {unknown}")
    covered = sum(1 for n in sites if hits.get(n, 0) > 0)
    print(f"\n{covered}/{len(sites)} draw sites in map_field.cpp hit, {sum(hits.values())} draws in {len(paths)} cases.")
    print("\nDraws outside the Fields files:")
    for s, c in foreign.most_common():
        print(f"- {s}: {c}")


if __name__ == "__main__":
    main()
