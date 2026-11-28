//! Dice sources at BN's helper level (`src/rng.h`).
//!
//! ADR 0004: no rule reads a global RNG. Each rule takes a `&mut dyn Dice` and
//! calls the same helper BN calls, with the same arguments, in the same order.
//! Every method here is one **top-level** helper call (`d == 0` in a
//! `bn-draws/1` case), so a replay source can answer each call from a recorded
//! case without knowing how BN composes helpers internally.
//!
//! [`BnDice`] composes the helpers the way `rng.cpp` does (`one_in` → `rng`,
//! `dice` → `rng`, `x_in_y` → `rng_float`, `roll_remainder` → `x_in_y`) over a
//! pluggable [`Engine`]. It is for running the code, not for matching BN's
//! stream: the engine is not mt19937 + libstdc++ distributions.

/// One BN dice helper per method. Arguments are exactly what the caller passes.
pub trait Dice {
    /// `rng(lo, hi)`: uniform int in `[min, max]` (BN swaps `lo > hi`).
    fn rng(&mut self, lo: i32, hi: i32) -> i32;
    /// `rng_float(lo, hi)`.
    fn rng_float(&mut self, lo: f64, hi: f64) -> f64;
    /// `one_in(chance)`: true if `chance <= 1` (no draw inside), else `rng(0, chance-1) == 0`.
    fn one_in(&mut self, chance: i32) -> bool;
    /// `x_in_y(x, y)`: `rng_float(0, 1) <= x / y`.
    fn x_in_y(&mut self, x: f64, y: f64) -> bool;
    /// `dice(number, sides)`: sum of `number` × `rng(1, sides)`.
    fn dice(&mut self, number: i32, sides: i32) -> i32;
    /// `roll_remainder(value)`: probabilistic rounding.
    fn roll_remainder(&mut self, value: f64) -> i32;

    /// A call out of Fields into another context (e.g. `item::burn`) that may
    /// draw on the other side. A replay source returns the recorded call: its
    /// inputs (`args`) for the stub to check, its outputs (`ret`) for the stub
    /// to apply, and it consumes every draw the real call made. Live sources
    /// return `None` and the stub simulates (session 3, ADR 0004).
    fn foreign(&mut self, _name: &str) -> Option<ForeignCall> {
        None
    }
}

/// A recorded call out of Fields (`call` / `ret` marks of a `bn-draws/1` case).
/// Numbers in `args` / `ret` (booleans are 0.0 / 1.0), strings in `sargs`
/// (from the `call` mark), and the objects of a list field of the `ret`
/// mark (e.g. `item::burn`'s `products`) in `list`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ForeignCall {
    pub name: String,
    pub args: Vec<(String, f64)>,
    pub ret: Vec<(String, f64)>,
    pub sargs: Vec<(String, String)>,
    pub list: Vec<Vec<(String, f64)>>,
}

impl ForeignCall {
    pub fn arg(&self, k: &str) -> f64 {
        self.args.iter().find(|(n, _)| n == k).unwrap_or_else(|| panic!("{}: no arg {k}", self.name)).1
    }
    pub fn sarg(&self, k: &str) -> &str {
        &self.sargs.iter().find(|(n, _)| n == k).unwrap_or_else(|| panic!("{}: no string arg {k}", self.name)).1
    }
    pub fn ret(&self, k: &str) -> f64 {
        self.ret.iter().find(|(n, _)| n == k).unwrap_or_else(|| panic!("{}: no ret {k}", self.name)).1
    }
}

/// `random_entry(container)` / `random_entry_removed(container)` pick an index
/// with `rng(0, size - 1)`. Shared so every caller makes the same draw.
pub fn random_index(d: &mut dyn Dice, len: usize) -> usize {
    d.rng(0, len as i32 - 1) as usize
}

/// `random_entry_removed`: draw an index, remove and return that element
/// (order of the rest kept, like `std::list::erase` / `std::vector::erase`).
pub fn random_entry_removed<T>(d: &mut dyn Dice, v: &mut Vec<T>) -> T {
    let i = random_index(d, v.len());
    v.remove(i)
}

/// `random_entry(container)`: default value if empty (no draw), else one draw.
pub fn random_entry<T: Clone + Default>(d: &mut dyn Dice, v: &[T]) -> T {
    if v.is_empty() {
        return T::default();
    }
    v[random_index(d, v.len())].clone()
}

/// A raw 32-bit engine.
pub trait Engine {
    fn next_u32(&mut self) -> u32;
}

/// SplitMix64, truncated to 32 bits. Good enough to exercise the code.
#[derive(Clone, Debug)]
pub struct SplitMix(pub u64);

impl Engine for SplitMix {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }
}

/// BN's helper composition (`rng.cpp`) over an [`Engine`].
pub struct BnDice<E: Engine> {
    pub engine: E,
}

impl<E: Engine> BnDice<E> {
    pub fn new(engine: E) -> Self {
        Self { engine }
    }

    fn raw_rng(&mut self, mut lo: i32, mut hi: i32) -> i32 {
        if lo > hi {
            std::mem::swap(&mut lo, &mut hi);
        }
        let span = (hi as i64 - lo as i64 + 1) as u64;
        // Rejection sampling over 32-bit draws (unbiased; not libstdc++'s algorithm).
        let zone = (u32::MAX as u64 + 1) / span * span;
        loop {
            let v = self.engine.next_u32() as u64;
            if v < zone {
                return (lo as i64 + (v % span) as i64) as i32;
            }
        }
    }

    fn raw_float(&mut self, mut lo: f64, mut hi: f64) -> f64 {
        if lo > hi {
            std::mem::swap(&mut lo, &mut hi);
        }
        let u = self.engine.next_u32() as f64 / (u32::MAX as f64 + 1.0);
        lo + u * (hi - lo)
    }

    fn raw_x_in_y(&mut self, x: f64, y: f64) -> bool {
        self.raw_float(0.0, 1.0) <= x / y
    }
}

impl<E: Engine> Dice for BnDice<E> {
    fn rng(&mut self, lo: i32, hi: i32) -> i32 {
        self.raw_rng(lo, hi)
    }
    fn rng_float(&mut self, lo: f64, hi: f64) -> f64 {
        self.raw_float(lo, hi)
    }
    fn one_in(&mut self, chance: i32) -> bool {
        chance <= 1 || self.raw_rng(0, chance - 1) == 0
    }
    fn x_in_y(&mut self, x: f64, y: f64) -> bool {
        self.raw_x_in_y(x, y)
    }
    fn dice(&mut self, number: i32, sides: i32) -> i32 {
        let mut ret = 0;
        for _ in 0..number {
            ret += self.raw_rng(1, sides);
        }
        ret
    }
    fn roll_remainder(&mut self, value: f64) -> i32 {
        let integ = value.trunc();
        let frac = value - integ; // modf: same sign as value
        let mut out = integ;
        if value > 0.0 && value > integ && self.raw_x_in_y(frac, 1.0) {
            out += 1.0;
        } else if value < 0.0 && value < integ && self.raw_x_in_y(-frac, 1.0) {
            out -= 1.0;
        }
        out as i32
    }
}

/// A dice source that logs every top-level call; wraps another source.
/// Handy in tests to assert the draw sequence a rule makes.
pub struct LoggingDice<'a> {
    pub inner: &'a mut dyn Dice,
    pub log: Vec<String>,
}

impl<'a> LoggingDice<'a> {
    pub fn new(inner: &'a mut dyn Dice) -> Self {
        Self { inner, log: Vec::new() }
    }
}

impl Dice for LoggingDice<'_> {
    fn rng(&mut self, lo: i32, hi: i32) -> i32 {
        let r = self.inner.rng(lo, hi);
        self.log.push(format!("rng({lo},{hi})={r}"));
        r
    }
    fn rng_float(&mut self, lo: f64, hi: f64) -> f64 {
        let r = self.inner.rng_float(lo, hi);
        self.log.push(format!("rng_float({lo},{hi})={r}"));
        r
    }
    fn one_in(&mut self, chance: i32) -> bool {
        let r = self.inner.one_in(chance);
        self.log.push(format!("one_in({chance})={r}"));
        r
    }
    fn x_in_y(&mut self, x: f64, y: f64) -> bool {
        let r = self.inner.x_in_y(x, y);
        self.log.push(format!("x_in_y({x},{y})={r}"));
        r
    }
    fn dice(&mut self, number: i32, sides: i32) -> i32 {
        let r = self.inner.dice(number, sides);
        self.log.push(format!("dice({number},{sides})={r}"));
        r
    }
    fn roll_remainder(&mut self, value: f64) -> i32 {
        let r = self.inner.roll_remainder(value);
        self.log.push(format!("roll_remainder({value})={r}"));
        r
    }
    fn foreign(&mut self, name: &str) -> Option<ForeignCall> {
        let r = self.inner.foreign(name);
        self.log.push(format!("foreign({name})={}", r.is_some()));
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_keep_bn_edge_cases() {
        let mut d = BnDice::new(SplitMix(1));
        assert!(d.one_in(1) && d.one_in(0) && d.one_in(-5));
        assert_eq!(d.roll_remainder(3.0), 3);
        assert_eq!(d.roll_remainder(0.0), 0);
        for _ in 0..1000 {
            let r = d.rng(5, -2);
            assert!((-2..=5).contains(&r));
            let s = d.dice(2, 6);
            assert!((2..=12).contains(&s));
        }
        assert_eq!(d.dice(0, 6), 0);
    }
}
