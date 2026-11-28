//! [`ReplayDice`]: a dice source that answers from a recorded case.
//!
//! It reads only `d == 0` draws, in file order (CASE_FORMAT.md). Every call
//! the Rust code makes must be the next recorded draw with the same helper and
//! the same arguments (exact `f64` equality); the recorded result is returned.
//! Any difference in function, arguments or count panics with the turn, the
//! draw index and BN's call site. Calls out of Fields (`call` .. `ret` marks)
//! are answered by [`Dice::foreign`], which also consumes the draws the real
//! call made on the other side.

use crate::case::{foreign_call, Case, Draw, Event};
use bn_dice::{Dice, ForeignCall};

pub struct ReplayDice<'c> {
    case: &'c Case,
    pos: usize,
    turn: i32,
    /// Matched draws so far (for the error context).
    matched: u64,
    recent: Vec<String>,
}

impl<'c> ReplayDice<'c> {
    pub fn new(case: &'c Case) -> Self {
        Self { case, pos: 0, turn: 0, matched: 0, recent: vec![] }
    }

    fn ctx(&self) -> String {
        format!(
            "case {} seed {} turn {} (after {} matched draws; last: [{}])",
            self.case.name,
            self.case.seed,
            self.turn,
            self.matched,
            self.recent.join("; ")
        )
    }

    fn fail(&self, what: String) -> ! {
        panic!("replay mismatch: {what}\n  at {}", self.ctx());
    }

    fn peek(&self) -> Option<&'c Event> {
        self.case.events.get(self.pos)
    }

    /// The harness starts turn `t`.
    pub fn begin_turn(&mut self, t: i32) {
        match self.peek() {
            Some(Event::Turn(n)) if *n == t => {
                self.pos += 1;
                self.turn = t;
            }
            other => self.fail(format!("expected start of turn {t}, case has {other:?}")),
        }
    }

    /// The harness ends turn `t`: every recorded draw of the turn must be used.
    pub fn end_turn(&mut self, t: i32) {
        match self.peek() {
            Some(Event::EndTurn(n)) if *n == t => self.pos += 1,
            Some(Event::Draw(d)) => self.fail(format!(
                "Rust made too few draws in turn {t}: BN's next draw is {}({}) = {} at {}",
                d.func,
                fmt_args(&d.args),
                d.r,
                d.src
            )),
            other => self.fail(format!("expected end of turn {t}, case has {other:?}")),
        }
    }

    /// Consume what a call BN makes virtually (so the recorder cannot bracket
    /// it: `deal_damage`) did on the other side: the run of draws and wrapped
    /// calls whose BN call site is not Fields code. Fields' own call sites are
    /// `map_field.cpp`, `rng.h`'s templates and `calendar.cpp`'s
    /// `rng(time_duration, ...)`.
    pub fn skip_foreign_draws(&mut self) -> usize {
        fn fields_own(src: &str) -> bool {
            src.starts_with("src/map/map_field.cpp") || src.starts_with("src/rng.h") || src.starts_with("src/calendar.cpp")
        }
        let mut n = 0;
        loop {
            match self.peek() {
                Some(Event::Draw(d)) if !fields_own(&d.src) => self.pos += 1,
                Some(Event::Call(_, _, sa)) if sa.iter().any(|(k, v)| k == "src" && !fields_own(v)) => {
                    // skip the whole call .. ret block
                    let mut depth = 0;
                    loop {
                        match self.peek() {
                            Some(Event::Call(..)) => depth += 1,
                            Some(Event::Ret(..)) => {
                                depth -= 1;
                                if depth == 0 {
                                    self.pos += 1;
                                    break;
                                }
                            }
                            None => self.fail("unbalanced call/ret".into()),
                            _ => {}
                        }
                        self.pos += 1;
                    }
                }
                _ => break,
            }
            n += 1;
        }
        n
    }

    pub fn finished(&self) -> bool {
        self.pos == self.case.events.len()
    }

    fn next_draw(&mut self, func: &str, args: &[f64]) -> &'c Draw {
        let got = format!("{func}({})", fmt_args(args));
        match self.peek() {
            Some(Event::Draw(d)) => {
                let same_args =
                    d.args.len() == args.len() && d.args.iter().zip(args).all(|(a, b)| a == b || (a.is_nan() && b.is_nan()));
                if d.func != func || !same_args {
                    self.fail(format!(
                        "Rust called {got}, BN called {}({}) at {} (draw i={})",
                        d.func,
                        fmt_args(&d.args),
                        d.src,
                        d.i
                    ));
                }
                self.pos += 1;
                self.matched += 1;
                self.recent.push(format!("{got}={}", d.r));
                if self.recent.len() > 4 {
                    self.recent.remove(0);
                }
                d
            }
            Some(Event::EndTurn(_)) => self.fail(format!("Rust made an extra draw {got}: BN drew nothing more this turn")),
            Some(Event::Call(name, ..)) => self.fail(format!("Rust called {got}, BN called out of Fields to {name} first")),
            other => self.fail(format!("Rust called {got}, case has {other:?}")),
        }
    }
}

fn fmt_args(a: &[f64]) -> String {
    a.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(",")
}

impl Dice for ReplayDice<'_> {
    fn rng(&mut self, lo: i32, hi: i32) -> i32 {
        self.next_draw("rng", &[lo as f64, hi as f64]).r as i32
    }
    fn rng_float(&mut self, lo: f64, hi: f64) -> f64 {
        self.next_draw("rng_float", &[lo, hi]).r
    }
    fn one_in(&mut self, chance: i32) -> bool {
        self.next_draw("one_in", &[chance as f64]).r != 0.0
    }
    fn x_in_y(&mut self, x: f64, y: f64) -> bool {
        self.next_draw("x_in_y", &[x, y]).r != 0.0
    }
    fn dice(&mut self, number: i32, sides: i32) -> i32 {
        self.next_draw("dice", &[number as f64, sides as f64]).r as i32
    }
    fn roll_remainder(&mut self, value: f64) -> i32 {
        self.next_draw("roll_remainder", &[value]).r as i32
    }
    fn foreign(&mut self, name: &str) -> Option<ForeignCall> {
        let (args, sargs) = match self.peek() {
            Some(Event::Call(n, a, sa)) if n == name => (a.clone(), sa.clone()),
            Some(Event::Draw(d)) => self.fail(format!(
                "Rust called out of Fields to {name}, BN drew first: {}({}) at {}",
                d.func,
                fmt_args(&d.args),
                d.src
            )),
            other => self.fail(format!("Rust called out of Fields to {name}, case has {other:?}")),
        };
        self.pos += 1;
        // Skip what the real call drew, up to its ret (calls may nest).
        let mut depth = 0;
        loop {
            match self.peek() {
                Some(Event::Call(..)) => depth += 1,
                Some(Event::Ret(n, r, l)) if depth == 0 => {
                    assert_eq!(n, name, "unbalanced call/ret in case");
                    let (r, l) = (r.clone(), l.clone());
                    self.pos += 1;
                    self.recent.push(format!("{name}(..)"));
                    return Some(foreign_call(name, args, sargs, r, l));
                }
                Some(Event::Ret(..)) => depth -= 1,
                Some(Event::Draw(_)) => {}
                other => self.fail(format!("call to {name} has no ret before {other:?}")),
            }
            self.pos += 1;
        }
    }
}
