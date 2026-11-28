# Observable dump format `bn-fields-dump/1`

The Observable dump is the state of every field entry in a region, written so a
Rust dump can produce **byte-identical** output from the same state. It is
written by `fields_recorder` (C++) and, from session 3, by the Rust port.

JSON lines, one object per line. A dump file may hold several snapshots (the
recorder writes one at turn 0 and one after the last turn); each snapshot is a
`dump` line followed by its `f` lines.

```json
{"k":"dump","format":"bn-fields-dump/1","seed":7,"turn":0,"region":[3,3],"entries":6}
{"k":"f","x":12,"y":12,"t":"fd_fire","n":2,"age":1,"alive":true}
```

| field | meaning |
|---|---|
| `seed` | engine seed of the run |
| `turn` | turns processed so far (0 = the seeded layout) |
| `region` | `[w, h]` in submaps (12×12 tiles each); tiles are `0..12w-1`, `0..12h-1` |
| `entries` | number of `f` lines that follow |
| `x`, `y` | region tile coordinates (`x = 12*submap_x + local_x`) |
| `t` | field type string id (`fd_*`) |
| `n` | intensity |
| `age` | age in whole turns (`to_turns<int>`); may be negative for fire |
| `alive` | `is_field_alive()`; dead entries are removed on the next processing pass |

Rules a matching writer must follow:

1. `f` lines are sorted by `(y, x, t)`, with `t` compared as a byte string.
2. Every field entry present on a tile is written, dead or alive.
3. Numbers are plain decimal integers; no spaces; key order exactly as above.

## Region and run conventions (both sides must use them)

- The region sits at absolute submap `(200, 200, 0)` in a private dimension
  `spike_fields`, which the Reality bubble never requests, so BN takes the
  "outside the bubble" path.
- The clock starts at `calendar::turn_zero + 1 hour`, one calendar turn per tick
  (`calendar_turns_this_tick = 1`).
- Each turn processes the region's submaps in **row-major order** (y, then x),
  skipping submaps with `field_count == 0`, then advances `calendar::turn` by
  one turn. This is the harness's order, not necessarily the in-game order of
  `mapbuffer::for_each_simulated_submap`.
- The engine is seeded with `rng_set_engine_seed(seed)` after the layout is
  built and before turn 1.
