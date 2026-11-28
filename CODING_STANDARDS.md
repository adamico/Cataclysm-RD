# Coding standards

Rules for the Rust code of Rusted Dawn that no tool checks. Each one is meant to be checked on a diff. Terms in bold are in the [Rusted Dawn glossary](GLOSSARY.md) and the [Game glossary](docs/game/GLOSSARY.md).

## Enforced elsewhere

Don't restate these. Follow the source.

- **Formatting and lints:** `rustfmt.toml`, `[workspace.lints]` in the workspace `Cargo.toml` (`unsafe_code` is forbidden; Clippy's pedantic group, `unwrap_used` and `expect_used` are on), `clippy.toml` (unwrap and expect allowed in tests), `rust-toolchain.toml`. Markdown and YAML: `.prettierrc`.
- **Crate boundaries, test placement and layering:** the workspace's crates README and its CI checks. The layers are decided in [ADR 0006](docs/adr/0006-consumer-owned-ports.md).
- **BN provenance in docs, Quirks recorded in an ADR, public names from the glossaries:** `tools/rd-checks`. Add a missing term to the Game glossary, or a word to `tools/vocabulary.txt`.
- **Randomness and tests against the Reference build:** [ADR 0004](docs/adr/0004-replay-recorded-draws.md). No rule reads a global RNG, replay tests drive a context's public interface, and they are written by a separate agent.

## Comments

- Don't explain code with comments. If a passage needs one, refactor it until it doesn't: extract and name a function, a type or a constant.
- Put rustdoc only on a context's public interface: its **Ports** and public types. Describe the contract in the language of the glossaries.
- Record decisions in an ADR under `docs/adr/`: a divergence from BN, a kept **Quirk**, a design choice.

## Names

- Use BN's name only for a concept the glossary doesn't cover, and spell it out in Rust style: `Terrain`, not `ter`; `SubmapPos`, not `SmPos`.
- Don't abbreviate, and don't keep BN's globals as they are (`fd_fire`, `#[allow(non_snake_case)]`).

## Errors

- **Machinery** that reads input it doesn't control (loading **Content**, save/load, recorded cases) returns `Result`.
- A rule never panics on game state. Where BN logs a debugmsg and carries on, give the fallback a name and log it.
- `panic!` belongs in tests and harnesses, and in code guarding an invariant that the types can't express. An `#[allow(clippy::unwrap_used)]` or `expect_used` outside tests needs the same reason.

## Quirks

- Reproduce a **Quirk** in a `quirks` module, behind a name that says it is deliberate (`LibstdcxxHeapOrder`).
- An **Approximation** is not a Quirk. It goes behind an option and is flagged, as the glossary says.

## Content

- A context crate owns its **Content** types and may derive `serde::Deserialize` on them.
- It never walks raw JSON values. Finding files, mod order and int ids are **Machinery** ([ADR 0006](docs/adr/0006-consumer-owned-ports.md)).
