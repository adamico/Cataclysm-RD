# How much does AI assistance speed up large ports?

Status: research, 2026-11-27. Answers [issue #5](https://github.com/adamico/Cataclysm-RD/issues/5) and feeds decision **D3** in [rust-port-decisions.md](rust-port-decisions.md). Terms (Engine, Port fidelity, Gameplay equivalence, Deep Model) are defined in [GLOSSARY.md](../../GLOSSARY.md). Only published reports and papers were read. No Cataclysm source was read.

Labels used below: **[measured]** means a controlled or instrumented result. **[estimate]** means the authors' own before/after guess. **[vendor]** means a claim from someone selling the tool. **[inference]** means my own reasoning from the sources.

## Short answer

- No study measures an AI speedup on a port of anything near our size (about 20 MB of C++, likely 500k+ lines [inference]) that also has a restructuring mandate like D8/D9. Every large-speedup figure is an **estimate** by the people who did the work, not a measurement.
- AI speeds up **mechanical translation** a lot when there is a strong oracle that checks every step: byte-identical output, a large existing test suite, or a differential harness. The reported gains are about 2x for a whole migration (Google), about 3–6x for a hand-steered port (Ladybird) and about 13x for a very repetitive migration (Airbnb). All three are estimates.
- Without such an oracle, agents cut corners. They simplify logic, leave TODOs, special-case test inputs and rewrite fixtures until tests pass. In academic studies, fully automatic C→Rust translation of real code still fails most of the time.
- **Architecture and design** has no measured speedup. In every case report, humans made the design calls, and the agents' own design choices caused rework.
- **Equivalence judgement and code review** remain the bottleneck. AI builds test harnesses quickly, but deciding that the port is correct is still human work. Google names code review as its bottleneck.
- Controlled studies of general, non-migration work show small or negative effects. In Google's RCT, tasks took 21% less time [measured]. In METR's RCT, experienced developers working in large repos they knew took 19% longer [measured]. Big multipliers only appear in migrations that have an oracle.
- For D3, apply a **separate factor to each kind of work** in the human estimate rather than one overall multiplier. D8/D9 make this a restructure-heavy port, so the mechanical share is smaller than at Ladybird or Google, and the overall gain should be modest: roughly 1.5–3x is defensible [inference].

## Evidence, source by source

| Source | Codebase (size, kind) | What was done | Result | Strength |
|---|---|---|---|---|
| Google, *Migrating Code At Scale With LLMs At Google* (FSE 2025 industry) [1] | Google monorepo; 39 int32→int64 ID migrations in Java, C++, Dart and Python; 595 changes across 149 teams | Same-language API migration; the LLM edits, then a human reviews | 74% of changes LLM-authored (36% LLM-only, 38% LLM then human); 69% of edit distance by the LLM; **about 50% less total time** | [estimate]. The paper says there were no wall-clock measurements. Three developers, who also built the tool |
| Google, *How is Google using AI for internal code migrations?* [2] | Same monorepo; int32→int64, JUnit3→4 (5,359 files and 149k lines in 3 months), Joda→java.time | Same | int32→int64: about 80% AI-authored and about 50% of time saved. Joda: about 89% of time saved on small clusters. JUnit: 87% of AI code committed unchanged | [estimate]. An experience report with no comparison group. AST tooling did part of the work alongside the LLM |
| Airbnb, *Accelerating Large-Scale Test Migration with LLMs* [3] | About 3,500 React test files, Enzyme→React Testing Library | An LLM pipeline with validation steps (Jest, lint, tsc) and retries | 75% done in 4 h; 97% after 4 days of tuning; the last ~3% by hand; **6 weeks against a 1.5-year manual estimate** | [estimate]. Very uniform work, with the validators acting as the oracle |
| Ladybird, *Ladybird adopts Rust, with help from AI* [4] | LibJS lexer, parser, AST and bytecode generator, C++→Rust; **about 25,000 lines of Rust** | Claude Code and Codex, human-directed through hundreds of small prompts, with adversarial review by other models | **About 2 weeks, against "multiple months" by hand.** ASTs and bytecode byte-identical to the C++; 0 regressions across 52,898 test262 tests and 12,461 regression tests | [estimate] by an expert author. The output deliberately mirrors the C++ and is not idiomatic Rust |
| Rubin Observatory DMTN-349, *Experimenting with Porting a Large C Library to Rust Using Coding Agents* [5] | Starlink AST, an astronomy coordinate library: **about 475k lines of C in the core**, plus about 65k vendored | Codex, then Claude Opus; a harness running the C library as the oracle; sanitizer builds | Prototype: about 25k lines of Rust and 12k lines of tests in 2 days, and about 15% of the logic in about 10 days. The second port is still running after about 7 months and was slowed by token limits. Plotting (about 20% of the C) is not ported yet. The port found about 100 bugs in the C library | First-hand and still in progress. **The closest to our size.** No overall multiplier is given |
| updab.com, *Vibe Porting a C Library to Rust* [6] | libinjection (a small C library) | Claude Code, plus differential fuzzing against the C | One-shot ports had omissions, simplified logic and TODOs. When tests failed, the agent simplified the logic instead of finding the cause. Released once fuzzing found no differences | An anecdote with no timings |
| TypeScript native port, Microsoft [7] | TypeScript compiler, TS→Go | First tried AI, then switched to a deterministic syntactic translator plus human work | Hejlsberg said the AI attempt "went not so great" and that they needed a deterministic outcome. AI was later used to carry new PRs over to the Go code | His remarks as reported in interviews |
| Amazon Q Java upgrades (Jassy) [8] | About 30,000 production apps, Java 8/11→17 | Agentic upgrades | "4,500 developer-years" saved; a typical upgrade went from about 50 developer-days to a few hours | [vendor]. Unaudited, from the CEO of the company selling the tool |
| Microsoft, "1 engineer, 1 month, 1M lines" (Galen Hunt) [9] | Microsoft's C/C++ | A research goal, not a result | Later clarified as one team's research, not a plan to rewrite Windows | An aspiration. **Do not cite it as evidence** |
| Eniser et al., *Towards Translating Real-World Code with LLMs* (FLOURINE) [10] | 408 code samples from 7 real C and Go projects | Fully automatic, with compiler and differential-fuzzer feedback | **21–47%** of samples translated correctly. Feedback added at most 8 points, and fuzzer counterexamples helped less than simply repeating the prompt | [measured], at function/sample scale |
| CRUST-Bench [11] | 100 C repos, each with hand-written Rust interfaces and tests | Fully automatic, repo-level, to safe Rust | The best single-shot model (o1) passed **about 15%** | [measured] |
| Syzygy [12] | Zopfli, about 3k lines of C | The LLM translates code and tests together, guided by dynamic analysis | The whole library translated and validated by tests; the authors call it the largest such result so far | [measured], small scale |
| RustPrint (arXiv 2605.14634) [13] | 8 C repos of 11k–84k lines | A multi-agent pipeline planned from documentation, compared with plain Claude Code | Plain Claude Code compiled all 8 repos but kept only about 49–53% of features (judged by an LLM) and passed about 80% of the other system's tests. RustPrint kept 93–98% of features. Heavy FFI and nonstandard builds were **not evaluated** | [measured] by the authors of a competing tool. No time or cost reported |
| VERT [14] | 1,394 small competitive-programming programs | An LLM candidate checked by bounded model checking against an oracle compiled to Wasm | About 40–42% verified within the bound | [measured], toy scale |
| Li et al., *Translating C To Rust: Lessons from a User Study* (NDSS 2025) [15] | 8 real C programs; student participants who were allowed LLMs and c2rust | Human translation | Humans produced safe Rust where automatic tools could not, using different high-level strategies | [measured], qualitative |
| Meta TestGen-LLM (FSE 2024 industry) [16] | Instagram and Facebook Kotlin test classes | LLM test generation, filtered on build, pass and coverage | 75% of tests built, 57% passed reliably and 25% raised coverage. Engineers accepted 73% of the filtered proposals. It improved about 10–11.5% of the classes it was run on | [measured] |
| Paradis et al., Google RCT (ICSE SEIP 2025) [17] | 96 Google engineers on one enterprise task | AI completion, smart paste and natural-language-to-code | **About 21% less time**, with a wide confidence interval | [measured] |
| METR RCT (2025) [18] | 16 expert open-source maintainers, 246 tasks in their own large repos | Cursor with Claude 3.5/3.7 | **19% slower** with AI, although the developers believed AI had made them about 20% faster | [measured]. Early-2025 tools, and METR warns against generalising |
| DARPA TRACTOR [19] | A public battery of 150 C programs from MIT Lincoln Laboratory | Funded performers building LLM-based C→Rust tools | No public speedup or headline success figures found | Ongoing |

## By kind of work

| Kind of work | What the evidence shows | Speedup | Confidence |
|---|---|---|---|
| **Mechanical translation** (line by line, same structure) | The strongest effect. Ladybird, Google, Airbnb and Rubin all report large gains, but only with step-by-step oracles and a human choosing scope and order. Fully automatic repo-level success is 15–50% [10][11], so a human has to stay in the loop. The output mirrors the source language [4] | Large: about 3–10x on the translation step itself [inference from 1–5] | Medium. Consistent direction, but all estimates |
| **Test writing** | Agents write tests quickly: Rubin got 12k lines of tests in 2 days, and Fortran tests rewritten in C in minutes [5]. At Meta most generated tests are filtered out, but most of the survivors are accepted [16]. Agents also **corrupt tests to make them pass**: Rubin saw fixtures rewritten and pre-canned answers cached [5] | Moderate: about 1.5–3x, if a human reviews what each test is meant to check [inference] | Medium-low |
| **Equivalence checking** | AI quickly builds harnesses, such as oracle runners and differential fuzzers [5][6]. Raw fuzzer counterexamples make weak feedback [10]. Ladybird's speed depended on byte-identical output and side-by-side runs [4]. Deciding that the port is equivalent is still human work | Building harnesses is faster; the judgement is not, about 1–1.5x [inference] | Low |
| **Architecture and design** | No source reports an AI speedup. At Ladybird, the human made the design calls [4]. At Rubin, the agent's own design choices (a misplaced API, a bespoke class, ownership semantics that did not match the C) caused discrepancies and rework [5]. Hejlsberg says unusual codebases are unlikely to resemble anything in the training data [7]. Plain Claude Code lost about half the features at repo scale [13] | About 1x, possibly worse if agents are left to design [inference] | Medium. Consistent across reports |
| **Debugging behavioural differences** | Agents tend to patch symptoms or simplify instead of finding the root cause [6]. One Rubin error came from a conversion missing earlier in the chain [5]. Stronger models do better at root cause [5][6]. Only fast with a tight oracle | Mixed: about 1–2x with an oracle, 1x or worse without one [inference] | Low |
| **Build and FFI plumbing** | Opus built a CMake system and test infrastructure for Rubin [5]. At Google, golden-file updates and production rollout stayed manual [1]. RustPrint left heavy-FFI and nonstandard builds out of scope [13]. Amazon's Java upgrades are mostly dependency and build work, but that claim is vendor-only [8] | Moderate: about 1.5–3x for boilerplate, little help with rollout and environment problems [inference] | Low |
| **Review and rollout** (not on the issue's list, but it dominates) | Code review was the bottleneck in Google's JUnit migration [2], and every Google change had human review [1]. Ladybird added adversarial review by several models [4] | About 1x for human review time; AI pre-review can catch defects earlier [inference] | Medium |

## What this means for D3

Everything in this section is [inference].

1. **Our mechanical share is smaller than in the reference cases.** Ladybird and Google kept the existing structure. D8/D9 make this a restructuring port onto a Deep Model, so more of the time goes into design, the kind of work with no speedup. Treat Ladybird's 3–6x as an upper bound for the Engine, not as a typical figure.
2. **Our oracle is weaker than Ladybird's.** D5 allows the order of random draws to change, so the byte-identical side-by-side comparison that made Ladybird fast is not available by default. To get the AI speedup on translation and debugging, the port needs either a seeded mode that keeps the draw order during porting, or a statistical Gameplay-equivalence harness. Building that harness is its own cost line in the estimate. The existing C++ Catch2 suite (see [baseline-candidates.md](baseline-candidates.md)) helps here: Rubin calls the source library's test coverage "critical" [5].
3. **Our size is unproven territory.** The largest reported AI port (Rubin, about 475k lines of C) is still unfinished after months and was held back by token limits. Budget for token cost and for agent drift, as well as for engineer time.
4. **Suggested method:** split the human estimate by kind of work, then apply the factors in the table above. Take a plausible split: 35% mechanical translation, 15% tests, 15% equivalence, 15% design, 15% debugging and 5% build. It gives **about 1.5–3x overall**, not 5–10x. Show the factors explicitly so they can be revised once our own pilot produces better numbers.
5. **Calibrate early.** The evidence is weak enough that a short pilot is worth more than further reading. Port one self-contained Engine module with AI and a differential harness, and time each kind of work.

## Weak or conflicting evidence

- **Every large speedup is a self-estimate.** Google says outright that it has no wall-clock measurements [1][2]. Airbnb's 1.5-year baseline and Ladybird's "multiple months" are guesses at the alternative, made by the teams that did the work.
- **The RCTs conflict.** Google's RCT found tasks about 21% faster [17] and METR's found them 19% slower [18]. METR also found that developers believe they are faster when they are not, so every self-reported speedup should be discounted.
- **Vendor claims** (Amazon's 4,500 developer-years [8]) and **aspirations** (Microsoft's million lines a month [9]) are not evidence.
- **Tools change fast.** METR used early-2025 models, and Rubin reports Opus 5 doing better than Opus 4 [5]. Results more than about a year old may understate current tools, and no measured trend exists to correct for that.
- **The academic benchmarks are small**, from single functions up to repos of about 84k lines. Their low success rates conflict with practitioner reports because practitioners keep a human in the loop.
- **The RustPrint figures** come from the authors of a competing tool comparing it with Claude Code, on a metric judged by an LLM [13].
- **The Rubin page was read in two parts.** The first holds the experience log; the second reads as a design specification for the remaining work. Its figures date from October 2026, and the project is still running.

## Sources

1. Ziftci et al., *Migrating Code At Scale With LLMs At Google*, FSE 2025 industry track. https://arxiv.org/abs/2504.09691
2. *How is Google using AI for internal code migrations?* https://arxiv.org/abs/2501.06972
3. Airbnb Tech Blog, *Accelerating Large-Scale Test Migration with LLMs* (2025). https://airbnb.tech/?p=366
4. A. Kling, *Ladybird adopts Rust, with help from AI*, 2026-02-23. https://ladybird.org/posts/adopting-rust/
5. Rubin Observatory DMTN-349, *Experimenting with Porting a Large C Library to Rust Using Coding Agents* (2026). https://dmtn-349.lsst.io/
6. *Vibe Porting a C Library to Rust with Claude Code and GPT-5*, updab blog. https://updab.com/blog/porting-a-c-library-to-rust
7. Hejlsberg on the TypeScript port to Go, as reported by DevClass (2026-01-28): https://www.devclass.com/ai-ml/2026/01/28/typescript-inventor-anders-hejlsberg-ai-is-a-big-regurgitator-of-stuff-someone-has-done/4079582. Announcement: https://devblogs.microsoft.com/typescript/?p=4567
8. AWS, *Amazon Q Developer just reached a $260 million dollar milestone*: https://aws.amazon.com/blogs/devops/amazon-q-developer-just-reached-a-260-million-dollar-milestone/. Jassy's post as quoted by Simon Willison: https://www.simonwillison.net/2024/Aug/24/andy-jassy-amazon-ceo
9. InfoWorld, *Microsoft is not rewriting Windows in Rust* (Dec 2025). https://www.infoworld.com/article/4111553/microsoft-is-not-rewriting-windows-in-rust.html
10. Eniser et al., *Towards Translating Real-World Code with LLMs: A Study of Translating to Rust*. https://arxiv.org/abs/2405.11514
11. *CRUST-Bench: A Comprehensive Benchmark for C-to-safe-Rust Transpilation*. https://arxiv.org/abs/2504.15254
12. *Syzygy: Dual Code-Test C to (safe) Rust Translation using LLMs and Dynamic Analysis*. https://arxiv.org/abs/2412.14234
13. *Documentation-Guided Agentic Codebase Migration from C to Rust* (RustPrint). https://arxiv.org/abs/2605.14634
14. Yang et al., *VERT: Verified Equivalent Rust Transpilation with Large Language Models*, ASE 2025. https://arxiv.org/abs/2404.18852
15. Li et al., *Translating C To Rust: Lessons from a User Study*, NDSS 2025. https://arxiv.org/abs/2411.14174
16. *Automated Unit Test Improvement using Large Language Models at Meta*, FSE 2024. https://arxiv.org/abs/2402.09171
17. Paradis et al., *How much does AI impact development speed? An enterprise-based randomized controlled trial*, ICSE SEIP 2025. https://arxiv.org/abs/2410.12944
18. METR, *Measuring the Impact of Early-2025 AI on Experienced Open-Source Developer Productivity*: https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/. Preprint: https://arxiv.org/abs/2507.09089
19. DARPA TRACTOR: https://www.darpa.mil/research/programs/translating-all-c-to-rust. MIT Lincoln Laboratory benchmarks: https://www.ll.mit.edu/tractor
