# Incremental C++ to Rust ports: prior art

Status: research, 2026-11-27, for ticket #4. External prior art only: neither **Baseline**'s source was read. Terms are defined in [GLOSSARY.md](../../GLOSSARY.md). Feeds the open question R3 (incremental FFI port vs rewrite) in [rust-port-decisions.md](rust-port-decisions.md).

Question: what does prior art say about porting a large C++ codebase to Rust incrementally behind an FFI seam, compared with rewriting it, and what does that mean for a turn-based game **Engine**?

Every factual claim below carries its source. Lines marked **Inference** are this note's reasoning about Cataclysm, not something a source says.

## Answer in brief

1. **Incremental ports of whole programs have been finished, at about a tenth of our size.** fish-shell went from ~57K lines of C++ to ~75K lines of Rust in about two years from first PR to the 4.0 beta, with roughly 14 months of main work against a "half a year" estimate [F1]. Cataclysm's ~20 MB of C++ (see [baseline-candidates.md](baseline-candidates.md)) is roughly ten times larger. **Inference**: fish's 2.5x overrun on schedule is the most relevant calibration point for the Phase 0 estimate (D2, D3).
2. **The FFI seam is scaffolding, and it is costly scaffolding.** fish used autocxx, had to fork both autocxx and cxx, wrapped values in `unique_ptr` or copied them to satisfy the bindings (losing performance temporarily), and ported its most entangled subsystem (the ~13K-line reader) in large chunks specifically "to avoid throwaway FFI code" [F1][F2].
3. **Firefox never ported Firefox.** Oxidation replaced *components with small APIs* (MP4 parser, encoding, CSS style, WebRender) and leaves the decision for existing code to each team; Mozilla's own wiki lists "Crossing the C++/Rust boundary can be difficult" as a weakness [M1]. Stylo (160K C++ lines replaced by 85K Rust) was a rewrite of one component, brought in from Servo, not a line-by-line port [M2].
4. **Tooling: cxx is the only option that is safe, maintained and predictable; it handles templates and inheritance poorly by design.** cxx has no `std::map`/`unordered_map`/`Option` bindings yet, keeps non-trivial C++ types opaque behind `UniquePtr` or `Pin<&mut T>`, and does not model inheritance [X1][X2]. autocxx is now "no longer actively maintained" [X3]. bindgen does not support inline functions, template specialisation, or automatic constructor/destructor calls, and exceptions through it are undefined behaviour [X4].
5. **Shared mutable state is the hardest part of the seam.** Chromium found ~8% of its `base` APIs unbridgeable by cxx and called out C++'s "multiple mutable references" to ref-counted objects as needing `unsafe` on the Rust side [C1]. Stylo's calls back into C++ off the main thread hit thread-safety bugs in C++ caches and global mutable state [M2].
6. **Per-call FFI cost is small, but without cross-language LTO nothing inlines across the seam.** cxx adds no serialization or runtime checks; a crossing costs a native call [X2][X5]. But LLVM cannot inline across the language boundary unless both sides use clang with a matching LLVM and `-C linker-plugin-lto` [L1][L2]. **Inference**: a hot loop that calls a tiny C++ accessor (a tile lookup, a field query) per creature per turn loses inlining and the vectorisation that follows it; the seam must sit outside the hot loops, not inside them.
7. **Build cost is real and lasting.** fish tried to delete CMake and failed, because Cargo could not install its assets [F1]. Corrosion, the usual CMake-to-Cargo bridge, is pre-1.0 and its cxx integration is marked "not 100% production ready yet" [B1]. cxx itself says combining build systems "can be very difficult" [X6]. BN closed a 2023 Rust proof of concept with "won't happen till build system is unified" ([rust-port-decisions.md](rust-port-decisions.md), BN #3766).
8. **Rewrites succeed when they are ports in disguise and fail when they are greenfield.** TypeScript 7 is a port to Go precisely to keep "semantics and structure" the same as the old code [T1]. Ladybird's 2026 LibJS frontend move to Rust was AI-assisted, kept C++ patterns, and was gated on byte-identical ASTs and bytecode versus the C++ pipeline [LB1]. Netscape's from-scratch rewrite is the classic failure [J1].
9. **For a game, structural mismatch matters more than call cost.** TypeScript's team rejected Rust because the borrow checker fights cyclic, shared object graphs [T1]; Ladybird had first rejected Rust in 2024 for poor fit with C++-style OOP [LB1]. **Inference**: Cataclysm's Engine (global `game`/`map` singletons, creatures and items with back-pointers) is closer to that profile than fish was, which argues that D8 ("structure is free") and an FFI port pull in opposite directions.

## 1. fish-shell: the only finished whole-program C++ to Rust port

Source: the maintainers' retrospective "Fish 4.0: The Fish Of Theseus" [F1], the in-tree developer guide from the port period [F2], and the 2023 status announcement [F3].

**Timeline.** The "Rewrite it in Rust" PR opened 2023-01-28 and merged 2023-02-19. A last all-C++ release, 3.7.0, shipped in January 2024, the same month the last C++ was removed. 4.0 beta came out 2024-12-17 [F1]. In November 2023 the port was "mostly" done, 60–74% depending on how it is counted, with the reader/input/screen/pager knot and the build system still left [F3].

**Scale and people.** ~57K C++ lines became ~75K Rust lines plus ~400 lines of C; 2,604 commits from 200+ authors, with about seven doing most of the work [F1].

**Tooling.** The Rust crate built a static library linked into the C++ `libfish.a`, with CMake calling Cargo through Corrosion, cxx for basic interop and autocxx for using C++ types from Rust [F2]. The project carried forks of both cxx and autocxx, to add `wchar_t`/`wstring` support and to drop the `unsafe` requirement on pointer-taking calls, which they saw as noise in a temporary bridge [F1][F2].

**Seam friction** [F2]:
- constants, typedefs and aliases do not cross, so they were redefined in Rust;
- many non-POD types cannot be passed by value, so references, raw pointers or smart pointers were used;
- `std::vector` of `shared_ptr` and similar needed hand-written pointer-plus-length wrappers;
- temporary FFI string types had to be quarantined in FFI modules.

autocxx sometimes could not parse C++ constructs, and its generated code was large enough to make rust-analyzer very slow [F1].

**Order.** First a leaf with an existing Rust port (widechar width), then the self-contained builtins, then the core using one of three moves per component: add FFI glue and port the caller later; move the callee; or write the Rust version alongside and keep the C++ until its callers are gone. The large entangled reader went last, in big chunks [F1]. They deliberately kept the C++ structure so before/after could be compared to catch mistranslations [F1].

**Costs that outlived the seam** [F1]: unwrap/assert crashes from missed subtleties; slow LTO builds; Cygwin support lost (no Rust target); a partial failure to replace CMake with Cargo. In 2023 the team warned of possibly ~20% slower performance in some cases [F3]; the `unique_ptr`/copy workarounds cost performance during the port and "most of it" was recovered later [F1].

**What they say they gained** [F1]: staying shippable throughout, which allowed a C++ release in the middle; easier contributor setup; `Option` and explicit modules over pointers and `#include`; `Send`/`Sync` as a route to multithreading.

**Inference for Cataclysm.** fish is a CLI program with one main loop and modest shared state; it still needed to port its most entangled part in one go. Cataclysm's equivalent knots (the `map`, `game`, creature tracker) are likely to force the same, and will be larger.

## 2. Firefox Oxidation: components, not a port

Source: Mozilla's Oxidation wiki [M1], Rust blog on Stylo [M2], Mozilla Hacks on rewriting the style component [M3], Firefox source docs on interop [M4], and the LLVM blog on cross-language LTO [L1].

- **Scope.** New and fully rewritten components "should strongly favor Rust"; for existing ones language choice is "an engineering decision" left to teams. Rust fits "standalone components with small APIs", parsers of untrusted input and parallel workloads [M1]. Shipped pieces include the MP4 metadata parser (FF48), encoding_rs (FF56), Stylo (FF57), the prefs parser (FF60) and WebRender (FF67) [M1].
- **Stylo** replaced ~160K C++ lines with ~85K Rust lines, after "two previous attempts to parallelize its style system in C++" had failed [M2]. It "needs to call back into Firefox's C++ code a lot", and calls into C++ off the main thread exposed thread-safety bugs in C++ caches and global mutable state [M2].
- **Correctness is not free.** 51 of 69 historical style-system security bugs (73.9%) would have been impossible in Rust, but the rewrite reintroduced a previously fixed logic bug (bug 1420001) because a check was overlooked while the tests that would have caught it were disabled [M3].
- **Interop guidance.** Keep transferred data simple (integers, pointers, `nsTArray`/`ThinVec`), split hash maps into key and value lists, use bindgen for non-trivial C++ interfaces and cbindgen to expose Rust's C API; hand-written declarations are unchecked and a C++ signature change makes "crashes … likely" [M4]. Firefox shares one heap allocator across both languages [M4].
- **Inlining.** Before cross-language LTO, "the optimizer does not know the definition of functions inside of other compilation units", so small functions could not be inlined across the seam; Mozilla built the feature and shipped it in release builds on Windows, macOS and Linux, with CI checks that inlining happens [L1].

**Inference for Cataclysm.** Firefox's model is Rust for well-bounded subsystems, with C++ remaining the host. That maps onto a **Deep Model** seam (D9) only if each bounded context can be cut out with a small API; it does not support a goal of porting the whole **Engine** through an FFI seam.

## 3. cxx, autocxx, bindgen: limits

| | cxx [X1][X2][X5][X6] | autocxx [X3][X7] | bindgen [X4] |
|---|---|---|---|
| Model | Hand-written bridge in a shared "subset" of both languages, checked statically | Generates bindings from C++ headers, built on bindgen and cxx | Generates `unsafe` Rust FFI from C/C++ headers |
| Status | Maintained | "no longer actively maintained"; points new users to cxx or Crubit | Maintained; C++ support best-effort |
| Templates | Only built-in instantiations (`unique_ptr`, `vector`, `shared_ptr`); `std::map`, `unordered_map`, `Option` still "tbd" | Instantiations like `Sandwich<Ham>` may become opaque placeholders | Templates without specialisation only; no template functions or methods of template classes |
| Inheritance | Not modelled; opaque types only | Not documented | "for the most part; there are some outstanding bugs" |
| Non-trivial types | Opaque, behind `UniquePtr`/reference; mutation needs `Pin<&mut T>`; no `Send`/`Sync` unless asserted `unsafe` | Same, via cxx | Constructors and destructors are not called automatically |
| Inline functions | Need an out-of-line shim | Same | Not supported |
| Exceptions | `Result<T>` return maps to throw/catch | Via cxx | Uncaught exception is undefined behaviour |
| Overhead | "zero or negligible"; nothing serialized, allocated or checked at runtime | Same as cxx | Plain C call |

Chromium's interop study adds measured coverage: about 60% of `base` APIs use only cxx-supported types, a further ~12% might work with by-value `std::string`, ~20% need safe handling of pointer out-parameters, and ~8% cannot fit cxx's model, mostly structs with raw pointers passed by value [C1]. Macros, overloads, operators and templates are hard to bridge; pure-virtual observers need wrapper types; shared ref-counted objects with multiple mutable references need `unsafe` [C1]. Chromium wanted generated `cxx::bridge` code rather than hand-written bridges per API [C1], which is what autocxx tried to be before it stopped being maintained [X3].

**Inference for Cataclysm.** The Engine's public surface is heavy in what all three tools handle worst: templates (`string_id<T>`, `int_id<T>`, generic JSON readers), class hierarchies (`Creature` → `Character`/`monster`/`npc`), `std::map`/`unordered_map` fields, and a global mutable `game`/`map`. A cxx seam would mostly carry opaque handles plus hand-written C++ shims; every shim is throwaway code under the fish lesson [F1].

## 4. Build-system cost: CMake or Make plus Cargo

- **Corrosion** (`corrosion_import_crate()`) is the standard CMake bridge. It needs CMake 3.22+, is pre-1.0 so minor versions may break, does not detect the target triple when cross-compiling on Linux, shares Cargo's target directory across configurations (so differently featured builds of a dependency rebuild in turn), and its cxx integration is experimental and "not 100% production ready yet" [B1].
- **cxx outside Cargo** requires calling the `cxxbridge` CLI to generate headers and `.cc` files, with its version pinned to the `cxx` crate; all Rust must be combined into one `staticlib` because several are likely to conflict; and Rust and generated C++ depend on each other, so GNU ld may need extra flags [X6]. The page warns that mixing build systems "can be very difficult" [X6].
- **Cross-language LTO** needs clang for the C++ side, an LLVM version matched to rustc's, an LTO-capable linker (LLD preferred), and the same LTO mode on both sides [L2]. Initial setup was "cumbersome" even for Mozilla [L1].
- **Experience.** fish found Cargo's install support too simple, so CMake survived for scripts, docs and tests [F1]. BN rejected a Rust PoC until its build system is unified (BN #3766, in [rust-port-decisions.md](rust-port-decisions.md)).

**Inference for Cataclysm.** Both Baselines build with CMake and Make, often with GCC. A cxx seam that is meant to be fast needs a clang-only, LLD, matched-LLVM toolchain on every platform, and two build systems for the whole life of the port. A clean-room Rust **Engine** carries no such cost.

## 5. Hot loops crossing the seam

- **Per-crossing cost.** cxx crossings cost the same as "any native-to-native FFI call", with nothing serialized, allocated or runtime-checked [X5]. In practice a cxx call is a call to a generated `extern "C"` shim, which calls the real function.
- **Loss of inlining.** Without cross-language LTO the optimizer cannot see across the seam, and even with the build in place a `target-cpu` attribute mismatch between clang and rustc blocked inlining until fixed [L1]. **Inference**: in a loop such as "for each tile in the reality bubble, read terrain and field", a non-inlined accessor costs a call per iteration and, more importantly, prevents hoisting and vectorisation. That is exactly the **Dense area** workload the project wants to speed up.
- **Data marshalling.** Firefox moves lists as `nsTArray`/`ThinVec` and splits maps into key and value arrays [M4]; cxx can borrow slices but cannot pass `std::map` [X1]. **Inference**: batch data across the seam once per turn (or per subsystem step) rather than querying per entity.
- **Concurrency.** cxx gives opaque C++ types no `Send`/`Sync` [X2], and Stylo's off-main-thread calls into C++ exposed races in C++ caches and global state [M2]. **Inference**: any plan to parallelise **Simulation** in Rust while C++ still owns `map` or creature state inherits that risk.

**Inference, design rule.** Put seams at subsystem boundaries that are crossed a few times per turn, move each subsystem's *data* to Rust together with its *code*, and keep hot loops entirely on one side. This is the same discipline fish applied by porting entangled code in large chunks [F1].

## 6. Rewrites that succeeded or failed

| Project | Kind | Outcome | Lesson |
|---|---|---|---|
| Netscape 6 (1998–2000) | Greenfield rewrite | About three years without a major release; Spolsky calls it "the single worst strategic mistake that any software company can make" [J1] | Rewriting throws away the bug fixes encoded in old code |
| Firefox Stylo (2017) | Component rewrite from Servo | Shipped, parallel, 160K → 85K lines [M2]; one regressed logic bug [M3] | Works when a component has a clear boundary and the new design is the point |
| librsvg (2016–2020) | Incremental C → Rust, function by function | By 2019 almost all logic in Rust; C kept for public API stubs, GObject boilerplate, tests and the pixbuf loader [R1] | A C (not C++) seam is easy; the library kept its ABI throughout |
| fish-shell 4.0 (2023–2024) | Incremental C++ → Rust via autocxx/cxx | Finished, 2.5x over schedule [F1] | The seam is throwaway; port knots in big chunks |
| TypeScript 7 (2025–) | Port JS → Go, not Rust | ~10x faster `tsc` [T2]; chosen to keep "semantics and structure" [T1] | A port's target language must match the source's object graph; Rust did not |
| Ladybird LibJS (2026) | AI-assisted port of a C++ frontend to Rust, replacing an abandoned Swift effort | ~25K Rust lines in ~2 weeks; 0 regressions on 52,898 test262 tests; byte-identical ASTs and bytecode vs C++, run in lockstep [LB1] | An oracle (identical output vs the C++) plus a self-contained target makes AI-assisted porting fast; Swift failed on C++ interop [LB1] |

**Inference for Cataclysm.** The successful cases had either a small seam (Firefox components, librsvg's C ABI) or a strong equivalence oracle (Ladybird's byte-identical output, TypeScript's test suite). **Gameplay equivalence** (D5) allows the order of random draws to change, so a byte-identical oracle is not available for free; the **Benchmark save** and a differential harness would have to stand in for it.

## What this means for R3 (incremental vs rewrite)

All of this is **inference** from the sources above.

- **Arguments for incremental (cxx)**: it is the only proven route to finish a whole C++ program in Rust while staying playable [F1]; it gives a running differential oracle (C++ and Rust side by side), as Ladybird used [LB1]; and it de-risks the Phase 0 go/no-go (D2) by delivering usable pieces.
- **Arguments against**: D8 lets structure change freely and D9 wants seams on bounded contexts rather than the C++ layout, while fish's lesson was to *keep* the C++ structure during the port [F1]; the Engine's templates, hierarchies and global mutable state are what cxx handles worst [X1][X2][C1]; hot loops in **Dense areas** cannot cross the seam without cross-language LTO and a clang-only toolchain [L1][L2]; and the build cost lasts for the life of the port [B1][X6].
- **Middle path suggested by the prior art**: port behind a *coarse* seam (Firefox-style: one bounded context, small API, data owned by Rust), not file by file (fish-style). Use the C++ build as a differential oracle at the seam rather than as a host to grow into. Whether such coarse seams exist in either Baseline is the question for the architecture analysis, which this note does not answer.

## Sources

- [F1] fish-shell, "Fish 4.0: The Fish Of Theseus", 2024-12-28. https://fishshell.com/blog/rustport/
- [F2] fish-shell, `doc_internal/rust-devel.md` (developer guide during the port). https://github.com/fish-shell/fish-shell/blob/master/doc_internal/rust-devel.md
- [F3] fish-shell discussion #10123, "Rust port status", 2023-11-26. https://github.com/fish-shell/fish-shell/discussions/10123
- [M1] Mozilla wiki, "Oxidation". https://wiki.mozilla.org/Oxidation
- [M2] Manish Goregaokar, "Fearless Concurrency in Firefox Quantum", Rust blog, 2017-11-14. https://blog.rust-lang.org/2017/11/14/Fearless-Concurrency-In-Firefox-Quantum/
- [M3] Diane Hosfelt, "Implications of Rewriting a Browser Component in Rust", Mozilla Hacks, 2019-02-28. https://hacks.mozilla.org/2019/02/rewriting-a-browser-component-in-rust/
- [M4] Firefox source docs, "Rust/C++ interop". https://firefox-source-docs.mozilla.org/writing-rust-code/cpp-interop.html
- [L1] Michael Woerister et al., "Closing the gap: cross-language LTO between Rust and C/C++", LLVM blog, 2019-09. https://blog.llvm.org/2019/09/closing-gap-cross-language-lto-between.html
- [L2] rustc book, "Linker-plugin-based LTO". https://doc.rust-lang.org/rustc/linker-plugin-lto.html
- [X1] cxx book, "Built-in bindings". https://cxx.rs/bindings.html
- [X2] cxx book, "extern \"C++\"" (opaque types, `Pin`, `Send`/`Sync`). https://cxx.rs/extern-c++.html
- [X3] autocxx book, introduction (maintenance status). https://google.github.io/autocxx/
- [X4] bindgen user guide, "Generating Bindings to C++". https://rust-lang.github.io/rust-bindgen/cpp.html
- [X5] cxx book, "Context" (overhead, comparison with bindgen). https://cxx.rs/context.html
- [X6] cxx book, "Some other build system". https://cxx.rs/build/other.html
- [X7] autocxx book, "Workflow" (what it can't ingest). https://google.github.io/autocxx/workflow.html
- [C1] Chromium security, "Rust and C++ interoperability". https://www.chromium.org/Home/chromium-security/memory-safety/rust-and-c-interoperability/
- [B1] Corrosion documentation (v0.6). https://corrosion-rs.github.io/corrosion/print.html
- [T1] TypeScript team, "Why Go?", microsoft/typescript-go discussion #411, 2025-03. https://github.com/microsoft/typescript-go/discussions/411
- [T2] Anders Hejlsberg, "A 10x Faster TypeScript", TypeScript blog, 2025-03-11. https://devblogs.microsoft.com/typescript/typescript-native-port/
- [LB1] Andreas Kling, "Ladybird adopts Rust, with help from AI", 2026-02-23. https://ladybird.org/posts/adopting-rust/
- [R1] Federico Mena Quintero, "Librsvg is almost rustified now", 2019-01-10. https://viruta.org/librsvg-is-almost-rustified-now.html (blog index: https://viruta.org/tag/librsvg.html)
- [J1] Joel Spolsky, "Things You Should Never Do, Part I", 2000-04-06. https://www.joelonsoftware.com/2000/04/06/things-you-should-never-do-part-i/
- BN #3766 (Rust PoC closed): cited from [rust-port-decisions.md](rust-port-decisions.md).
