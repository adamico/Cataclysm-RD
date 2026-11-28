"""Compare a C++ and a Rust Fields-only soak run (issue #14, session 3).

    python3 -I spike/soak/compare_soak.py <bn-dir> <rust-dir> [--turns 25,50,100] [--alpha 0.05]

Both directories hold `seed-<s>.dump.jsonl` files (`bn-fields-dump/1`) for the
same layout, seed set and turn count; snapshots per turn (`--dump-every`).

Statistic. For every checked turn T, field type and metric, each seed gives one
number (the seeds are the independent units; tiles inside one run are not):
  tiles           alive entries of the type
  sum_intensity   sum of their intensities
  mean_intensity  sum_intensity / tiles
  mean_age        mean age in turns
A type absent from a run counts as tiles = sum = 0; mean_* are then left out.

Test. Two-sided exact permutation test on the difference of means (every
split of the pooled 10 + 10 values: C(20, 10) = 184,756), so no normality
assumption. Holm-Bonferroni over the whole family keeps the chance of any
false alarm at or below alpha.

Detection target (printed per row). With n = 10 seeds per side, a mean shift
of d pooled standard deviations is detected with ~80 % power when
d >= (z_{1-a/2} + z_{0.8}) * sqrt(2/10): d = 1.25 at a = 0.05 per test, and
d = 1.25 * (z_{1-a'/2} + 0.84) / 2.80 at the Holm-adjusted a' = alpha / m for
the first comparison. The 'detectable' column turns that into the metric's
own units using the pooled SD, i.e. the smallest real BN/Rust difference this
run can be expected to notice. Smaller translation errors pass unseen.
"""
import itertools
import json
import math
import os
import statistics
import sys
from collections import defaultdict


def load(path):
    snaps = {}
    head, rows = None, []
    for line in open(path):
        o = json.loads(line)
        if o["k"] == "dump":
            if head is not None:
                snaps[head["turn"]] = rows
            head, rows = o, []
        elif o["k"] == "f":
            rows.append(o)
    if head is not None:
        snaps[head["turn"]] = rows
    return snaps


def metrics(rows):
    agg = defaultdict(list)
    for r in rows:
        if r["alive"]:
            agg[r["t"]].append(r)
    out = {}
    for t, rs in agg.items():
        n = len(rs)
        s = sum(r["n"] for r in rs)
        out[t] = {"tiles": n, "sum_intensity": s, "mean_intensity": s / n,
                  "mean_age": sum(r["age"] for r in rs) / n}
    return out


def side(d, turns):
    """{(turn, type, metric): [value per seed]} plus the seed list."""
    files = sorted(f for f in os.listdir(d) if f.endswith(".dump.jsonl"))
    per = defaultdict(dict)
    seeds = []
    for f in files:
        seed = int(f.split("-")[1].split(".")[0])
        seeds.append(seed)
        snaps = load(os.path.join(d, f))
        for T in turns:
            per[seed][T] = metrics(snaps[T])
    return seeds, per


def perm_p(a, b):
    pooled = a + b
    n = len(a)
    obs = abs(statistics.fmean(a) - statistics.fmean(b))
    total = sum(pooled)
    hits = count = 0
    for idx in itertools.combinations(range(len(pooled)), n):
        sa = sum(pooled[i] for i in idx)
        diff = abs(sa / n - (total - sa) / (len(pooled) - n))
        if diff >= obs - 1e-12:
            hits += 1
        count += 1
    return hits / count


def z(p):
    # inverse normal CDF (Acklam-free: bisection on erf is enough here)
    lo, hi = -10.0, 10.0
    for _ in range(100):
        mid = (lo + hi) / 2
        if 0.5 * (1 + math.erf(mid / math.sqrt(2))) < p:
            lo = mid
        else:
            hi = mid
    return lo


def main():
    args = sys.argv[1:]
    bn_dir, rs_dir = args[0], args[1]
    turns = [25, 50, 100]
    alpha = 0.05
    if "--turns" in args:
        turns = [int(x) for x in args[args.index("--turns") + 1].split(",")]
    if "--alpha" in args:
        alpha = float(args[args.index("--alpha") + 1])
    bseeds, bn = side(bn_dir, turns)
    rseeds, rs = side(rs_dir, turns)
    assert bseeds == rseeds, (bseeds, rseeds)
    n = len(bseeds)
    rows = []
    for T in turns:
        types = sorted({t for s in bseeds for t in list(bn[s][T]) + list(rs[s][T])})
        for t in types:
            for m in ("tiles", "sum_intensity", "mean_intensity", "mean_age"):
                if m in ("tiles", "sum_intensity"):
                    a = [bn[s][T].get(t, {}).get(m, 0) for s in bseeds]
                    b = [rs[s][T].get(t, {}).get(m, 0) for s in bseeds]
                else:
                    a = [bn[s][T][t][m] for s in bseeds if t in bn[s][T]]
                    b = [rs[s][T][t][m] for s in bseeds if t in rs[s][T]]
                if len(a) < 3 or len(b) < 3:
                    continue
                if len(set(a + b)) == 1:
                    p = 1.0
                elif len(a) == len(b) == n:
                    p = perm_p([float(x) for x in a], [float(x) for x in b])
                else:
                    # unequal sizes (type absent in some runs): 20k random splits
                    import random
                    rnd = random.Random(1)
                    pooled = a + b
                    obs = abs(statistics.fmean(a) - statistics.fmean(b))
                    hits = 0
                    for _ in range(20000):
                        rnd.shuffle(pooled)
                        d = abs(statistics.fmean(pooled[:len(a)]) - statistics.fmean(pooled[len(a):]))
                        hits += d >= obs - 1e-12
                    p = hits / 20000
                sd = math.sqrt(((len(a) - 1) * statistics.pvariance(a) * len(a) / max(len(a) - 1, 1)
                                + (len(b) - 1) * statistics.pvariance(b) * len(b) / max(len(b) - 1, 1))
                               / max(len(a) + len(b) - 2, 1))
                rows.append([T, t, m, statistics.fmean(a), statistics.fmean(b), sd, p, len(a), len(b)])
    k = len(rows)
    order = sorted(range(k), key=lambda i: rows[i][6])
    reject = [False] * k
    for rank, i in enumerate(order):
        if rows[i][6] <= alpha / (k - rank):
            reject[i] = True
        else:
            break
    d_per = (z(1 - alpha / 2) + z(0.8)) * math.sqrt(2 / n)
    d_fam = (z(1 - alpha / k / 2) + z(0.8)) * math.sqrt(2 / n)
    print(f"seeds: {n} per side ({','.join(map(str, bseeds))}); comparisons: {k}; "
          f"family alpha {alpha} (Holm)")
    print(f"detection target: a mean shift of {d_per:.2f} pooled SD per test (alpha {alpha}), "
          f"{d_fam:.2f} SD after Holm's first step (alpha {alpha / k:.5f}), 80% power")
    print()
    print("| turn | type | metric | BN mean | Rust mean | pooled SD | detectable (Holm) | p | verdict |")
    print("|---|---|---|---|---|---|---|---|---|")
    for i, (T, t, m, ma, mb, sd, p, na, nb) in enumerate(rows):
        verdict = "**differs**" if reject[i] else "same"
        extra = "" if na == nb == n else f" (n {na}/{nb})"
        print(f"| {T} | {t} | {m} | {ma:.2f} | {mb:.2f} | {sd:.2f} | {d_fam * sd:.2f} | {p:.4f} | {verdict}{extra} |")
    nrej = sum(reject)
    print()
    print(f"{nrej} of {k} comparisons differ after Holm correction.")
    return 1 if nrej else 0


if __name__ == "__main__":
    sys.exit(main())
