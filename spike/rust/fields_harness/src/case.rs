//! `bn-draws/1` case files (`spike/recorder/CASE_FORMAT.md`).

use bn_dice::ForeignCall;
use serde_json::Value;

/// One `d == 0` draw.
#[derive(Clone, Debug)]
pub struct Draw {
    pub i: u64,
    pub func: String,
    pub args: Vec<f64>,
    pub r: f64,
    pub src: String,
}

/// What the replay walks through, in file order.
#[derive(Clone, Debug)]
pub enum Event {
    Draw(Draw),
    /// A call out of Fields: everything up to the matching `Ret` belongs to it.
    /// name, numeric args, string args (incl. `src`, BN's call site, when recorded)
    Call(String, Vec<(String, f64)>, Vec<(String, String)>),
    Ret(String, Vec<(String, f64)>, Vec<Vec<(String, f64)>>),
    Turn(i32),
    EndTurn(i32),
}

#[derive(Clone, Debug)]
pub struct Def {
    pub id: String,
    pub movecost: i32,
    pub flags: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ItemDef {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub itype: String,
    pub weight_g: i64,
    pub flammable: bool,
    pub rock: bool,
}

/// Non-field tile state after a turn (`tile` marks).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TileState {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub ter: String,
    pub furn: String,
    pub rad: i32,
    pub items: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Case {
    pub name: String,
    pub seed: u64,
    pub turns: i32,
    pub events: Vec<Event>,
    pub ter_defs: Vec<Def>,
    pub furn_defs: Vec<Def>,
    pub item_defs: Vec<ItemDef>,
    /// `tiles[t]`: tile states after turn `t` (0 = layout).
    pub tiles: Vec<Vec<TileState>>,
    pub bypass_steps: u64,
    /// Creature cases: BN's answers to the Combat queries (`creature_def`).
    pub creature_def: Option<Value>,
    /// Creature cases: (hp, moves, dead) after turn `t` (0 = placed).
    pub creature: Vec<(i32, i32, bool)>,
}

impl Case {
    pub fn is_creature_case(&self) -> bool {
        self.creature_def.is_some()
    }
}

fn num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap(),
        Value::Bool(b) => *b as i32 as f64,
        Value::String(s) if s == "inf" => f64::INFINITY,
        Value::String(s) if s == "-inf" => f64::NEG_INFINITY,
        Value::String(s) if s == "nan" => f64::NAN,
        _ => panic!("not a number: {v}"),
    }
}

/// String fields of a mark, minus the bookkeeping ones.
fn string_fields(o: &serde_json::Map<String, Value>) -> Vec<(String, String)> {
    o.iter()
        .filter(|(k, v)| !matches!(k.as_str(), "k" | "label" | "fn" | "pc") && v.is_string())
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
        .collect()
}

/// The first array-of-objects field of a mark, as numeric records.
fn list_field(o: &serde_json::Map<String, Value>) -> Vec<Vec<(String, f64)>> {
    o.values()
        .filter_map(Value::as_array)
        .flat_map(|a| a.iter().filter_map(Value::as_object).map(numeric_fields))
        .collect()
}

/// Numeric fields of a mark, minus the bookkeeping ones.
fn numeric_fields(o: &serde_json::Map<String, Value>) -> Vec<(String, f64)> {
    o.iter()
        .filter(|(k, v)| !matches!(k.as_str(), "k" | "before" | "label" | "fn") && (v.is_number() || v.is_boolean()))
        .map(|(k, v)| (k.clone(), num(v)))
        .collect()
}

impl Case {
    pub fn parse(text: &str) -> Case {
        let mut c = Case::default();
        for line in text.lines() {
            let v: Value = serde_json::from_str(line).unwrap_or_else(|e| panic!("bad case line {line:?}: {e}"));
            let o = v.as_object().unwrap();
            let s = |k: &str| o.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            let i = |k: &str| o.get(k).and_then(Value::as_i64).unwrap_or(0);
            match o["k"].as_str().unwrap() {
                "header" => {
                    c.name = s("case");
                    c.seed = i("seed") as u64;
                    c.turns = i("turns") as i32;
                }
                "draw" if i("d") == 0 => c.events.push(Event::Draw(Draw {
                    i: i("i") as u64,
                    func: s("fn"),
                    args: o["args"].as_array().unwrap().iter().map(num).collect(),
                    r: num(&o["r"]),
                    src: s("src"),
                })),
                "draw" => {}
                "bypass" => c.bypass_steps += i("steps").max(0) as u64,
                "mark" => match o["label"].as_str().unwrap() {
                    "turn" => c.events.push(Event::Turn(i("turn") as i32)),
                    "end_turn" => c.events.push(Event::EndTurn(i("turn") as i32)),
                    "call" => c.events.push(Event::Call(s("fn"), numeric_fields(o), string_fields(o))),
                    "ret" => c.events.push(Event::Ret(s("fn"), numeric_fields(o), list_field(o))),
                    "ter_def" | "furn_def" => {
                        let d = Def {
                            id: s("id"),
                            movecost: i("movecost") as i32,
                            flags: o["flags"].as_array().unwrap().iter().map(|f| f.as_str().unwrap().to_string()).collect(),
                        };
                        if o["label"] == "ter_def" {
                            c.ter_defs.push(d)
                        } else {
                            c.furn_defs.push(d)
                        }
                    }
                    "item_def" => c.item_defs.push(ItemDef {
                        x: i("x") as i32,
                        y: i("y") as i32,
                        z: i("z") as i32,
                        itype: s("itype"),
                        weight_g: i("weight_g"),
                        flammable: o["flammable"].as_bool().unwrap(),
                        rock: o["rock"].as_bool().unwrap(),
                    }),
                    "creature_def" => c.creature_def = Some(v.clone()),
                    "creature" => c.creature.push((i("hp") as i32, i("moves") as i32, o["dead"].as_bool().unwrap())),
                    "tile" => {
                        let t = i("turn") as usize;
                        while c.tiles.len() <= t {
                            c.tiles.push(vec![]);
                        }
                        c.tiles[t].push(TileState {
                            x: i("x") as i32,
                            y: i("y") as i32,
                            z: i("z") as i32,
                            ter: s("ter"),
                            furn: s("furn"),
                            rad: i("rad") as i32,
                            items: i("items") as usize,
                        });
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        while c.tiles.len() <= c.turns as usize {
            c.tiles.push(vec![]);
        }
        c
    }

    pub fn load(path: &std::path::Path) -> Case {
        Case::parse(&std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
    }

    /// Draw call sites (`src`) in this case, for coverage.
    pub fn draw_sites(&self) -> impl Iterator<Item = &str> {
        self.events.iter().filter_map(|e| match e {
            Event::Draw(d) => Some(d.src.as_str()),
            _ => None,
        })
    }
}

/// Turn a `Call` .. `Ret` pair into the value a stub sees.
pub fn foreign_call(
    name: &str,
    args: Vec<(String, f64)>,
    sargs: Vec<(String, String)>,
    ret: Vec<(String, f64)>,
    list: Vec<Vec<(String, f64)>>,
) -> ForeignCall {
    ForeignCall { name: name.to_string(), args, ret, sargs, list }
}
