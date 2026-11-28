# Case file format `bn-draws/1`

A case file is the record of every helper-level random draw made by untouched BN
code while the recorder ran. It is written by `rng_recording.cpp` (the recording
copy of BN's `src/rng.cpp`) and read by the Rust replay tests (session 3).

The file is **JSON lines**: one JSON object per line, UTF-8, `\n` separated. Every
object has a `k` (kind) field. Readers must ignore unknown kinds and unknown
fields, so later sessions can add them.

## Kinds

| `k` | when | fields |
|---|---|---|
| `header` | first line | `format` (`"bn-draws/1"`), `baseline` (BN commit), plus whatever the recorder program adds (`case`, `seed`, `turns`, `layout`, ...) |
| `seed` | `rng_set_engine_seed()` was called | `before` (draw index the seeding precedes), `seed` |
| `mark` | the recorder program segments the case | `before` (index of the next draw), `label`, extra fields (e.g. `turn`, `index`) |
| `draw` | a helper returned | see below |
| `bypass` | the engine moved outside any helper | `before` (index of the next draw), `steps` (raw engine steps; `-1` = not found within 10M steps, e.g. reseeded by hand) |
| `footer` | last line, written by `draw_recorder::end()` | `draws` (all depths), `top` (depth 0), `bypass_steps`, `unrecorded` |

### `draw`

```json
{"k":"draw","i":17,"d":0,"fn":"one_in","args":[4],"r":false,"pc":"0x2a1f3c0"}
{"k":"draw","i":18,"d":1,"fn":"rng","args":[0,3],"r":2}
```

- `i`: index assigned when the helper was **entered**, counting every recorded
  helper call at every depth, from 0.
- `d`: nesting depth. `0` is a call from game code; `1+` is a helper calling
  another helper inside `rng.cpp` (for example `one_in` → `rng`, `dice` → `rng`,
  `roll_remainder` → `x_in_y` → `rng_float`).
- `fn`: the helper's name, as in `rng.h`.
- `args`: the arguments **as passed by the caller** (before any `lo > hi` swap).
- `r`: the result.
- `pc` (depth 0 only): the return address, as an offset from the start of the
  recorder image. Resolve it to a BN source line with
  `addr2line -Cfie <recorder binary> <pc>` (the Reference build has debug info).

A line is written when the helper **returns**, so nested lines (`d >= 1`) come
before their parent in the file. **Replay tests read only `d == 0` lines, in
file order**: that is the exact sequence of draws the game code saw. Nested lines
are for debugging.

### Helpers and their `args` / `r`

| `fn` | `args` | `r` |
|---|---|---|
| `rng` | `lo, hi` (int) | int |
| `rng_float` | `lo, hi` (double) | double |
| `rng_bits` | — | unsigned int (also behind `weighted_list` picks) |
| `one_in` | `chance` | bool |
| `one_turn_in` | `turns` (int) | bool |
| `x_in_y` | `x, y` (double) | bool |
| `check` | `p` in one-in-a-million (int) | bool |
| `dice` | `number, sides` | int |
| `roll_remainder` | `value` (double) | int |
| `rng_normal` | `lo, hi` | double |
| `normal_roll` | `mean, stddev` | double |
| `rng_exponential` | `min, mean` | double |
| `exponential_roll` | `lambda` | double |
| `random_direction` | — | double, radians |

`random_entry`, `random_entry_ref`, `random_entry_removed`, `random_entry_opt`
and `random_entry_detached` are header templates: they appear as the `rng(0,
size-1)` draw they make.

Doubles are printed with `%.17g`, so they round-trip exactly to an `f64`.
Non-finite values are written as the strings `"inf"`, `"-inf"`, `"nan"`.

## What is not recorded

- **Bypasses.** Code that takes `rng_get_engine()` and draws from it directly
  (`std::shuffle(..., rng_get_engine())`, `distribution.cpp`, weather wind) or
  uses `rand()` is invisible at helper level. The recorder cannot see the values,
  but it does detect that the main engine moved between two helper calls and
  writes a `bypass` line with the number of raw steps. A case with any `bypass`
  line cannot be replayed helper by helper. `rand()` does not touch the engine
  and is not detected at all.
- **Worker threads.** Draws on threads that called `rng_set_worker_seed()` use a
  per-thread engine and are not recorded (counted in `unrecorded`). Fields
  processing is single-threaded.
- `std::normal_distribution` caches its second value, so the raw engine stream
  cannot be reproduced from a case; helper-level replay is unaffected.
