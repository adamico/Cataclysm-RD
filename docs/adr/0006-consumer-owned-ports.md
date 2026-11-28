# Each context owns the Ports it needs

Each bounded context is one crate, and it declares as Rust traits the **Ports** it needs from its neighbours, in its own language. **Adapters** (stubs included) implement them in crates of their own, and an app crate wires contexts to Adapters. A context crate never depends on another context crate, only on **Machinery** crates. Every crate states its layer, and a layering check in CI fails on any edge or external crate the layer table doesn't allow. This keeps each context building and testing alone behind stubs, which is how the Fields spike held Gameplay equivalence with seven neighbours stubbed, and how Monsters will be gated with Reality bubble stubbed ([ADR 0005](0005-gated-go-on-monsters.md)).

Decided while grilling the coding standards before [Month 1](https://github.com/adamico/Cataclysm-RD/issues/15).

## Considered options

- **The providing context publishes the trait** (Combat defines how Harm is dealt, and Monsters depends on Combat's interface). It is the obvious layout, but every context would then build against its neighbours' interfaces. Port order would follow dependency order, and a neighbour's interface change would ripple into contexts that use one call of it.
- **No layering check, review only.** It leaves the boundary to the person's reading of diffs, which ADR 0004 and issue 15 set out to avoid.

## Consequences

- **The layers and their allowed edges:**

  | Layer       | May depend on          | External crates               |
  | ----------- | ---------------------- | ----------------------------- |
  | `context`   | `machinery`            | `serde`                       |
  | `machinery` | `machinery`            | per crate, in the layer table |
  | `adapter`   | `context`, `machinery` | per crate, in the layer table |
  | `app`       | anything               | anything                      |

- **The same need can be declared twice.** Monsters and Characters may each declare a Port onto Combat. Two narrow Ports are preferred to one shared one, and an **Adapter** may serve both.
- **Content belongs to the context that speaks it.** A context crate owns its Content types and may derive `serde::Deserialize` on them. A Machinery `content` crate finds files, orders mods and assigns int ids, and a context never reads raw JSON.
- **A new external crate changes the layer table,** so it shows up in the pull request.
