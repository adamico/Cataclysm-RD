# Gated go: calibrate on Monsters before committing

Rusted Dawn goes ahead, one 60-hour month at a time. The Engine's AI-assisted estimate (≈ 9.5 person-months, about 22 months at 60 h a month) is calibrated on Fields alone, an easy context, and sits 35x below its pessimistic bound. So the port commits to two months, not 22. Month 1 is profiling, the Benchmark save, a first Turn throughput comparison (the spike's Rust Fields against BN's Fields on a field-dense lab), the harness, and the Monsters design. Month 2 rewrites **Monsters**, a hard context, behind the oracle. Its measurements are the gate.

Decided in [Go or no-go?](https://github.com/adamico/Cataclysm-RD/issues/12). The figures are in [estimate.md](../estimate.md).

## Triggers back to go/no-go

- **Any month's end:** each month ends with a result the person can see. If they don't want the next month, that is a no-go, recorded, not drifted into.
- **At the Monsters gate:**
  - The Monsters rates, extrapolated to the whole Engine, put the human time above the estimate's range.
  - The oracle doesn't hold: more than 3 times in Monsters, a defect escaped replay and the mutation gate, or the person had to read a function body to settle an escalation. That is the pessimistic bound's world, and it is a no-go at this budget.
  - Token cost per easy-equivalent k line, extrapolated to the whole Engine, needs more than the person's current subscription tier.

## Considered options

- **Plain go on the estimate.** It trusts a figure measured on one easy context. The tier multipliers, statistical-only code, Tangles and cross-context integration were not measured.
- **No-go.** It throws away a 35x advantage that two months can confirm or refute.
- **A calendar cap (21 person-months).** At this budget that is four to eight years. Interest runs out long before it, so a monthly decision replaces the cap.
- **Gating on another hard context.** Reality bubble is about 4 months on its own. Vehicles can't be ported without untangling Reality bubble. Turn loop needs every other context. Monsters covers plan/apply's restructured draws, the hordes Tangle and stubbed neighbours, all in about 120 h.

## Consequences

- **Port order starts with Monsters,** after month 1's setup. The rest of the order is spec work.
- **Reality bubble is stubbed behind ports for the gate,** the way the spike stubbed Fields' seven neighbours.
- **The Frontend gets its own go/no-go** once the headless Engine is done. Nothing is playable before that.
- **Token cost is measured from month 2.** The estimate never had it.
