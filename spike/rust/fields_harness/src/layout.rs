//! Layout files (`spike/recorder/README.md`), parsed like `fields_recorder.cpp`.

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldCell {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub id: String,
    pub intensity: i32,
    pub age: i32,
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub name: String,
    pub w: i32,
    pub h: i32,
    pub zlo: i32,
    pub zhi: i32,
    pub clock: i32,
    pub default_ter: String,
    pub ters: Vec<Cell>,
    pub furns: Vec<Cell>,
    pub items: Vec<Cell>,
    pub rads: Vec<(i32, i32, i32, i32)>,
    pub fields: Vec<FieldCell>,
    /// Creature cases (`monster <x> <y> <mon_id>`).
    pub mons: Vec<Cell>,
}

impl Layout {
    pub fn nz(&self) -> i32 {
        self.zhi - self.zlo + 1
    }

    pub fn parse(name: &str, text: &str) -> Layout {
        let mut l = Layout {
            name: name.to_string(),
            w: 1,
            h: 1,
            zlo: 0,
            zhi: 0,
            clock: 3600,
            default_ter: "t_dirt".into(),
            ters: vec![],
            furns: vec![],
            items: vec![],
            rads: vec![],
            fields: vec![],
            mons: vec![],
        };
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap();
            let t: Vec<&str> = line.split_whitespace().collect();
            if t.is_empty() {
                continue;
            }
            let n = |i: usize| -> i32 { t[i].parse().unwrap_or_else(|_| panic!("bad number in {raw:?}")) };
            let z = |i: usize| -> i32 { t.get(i).map_or(0, |s| s.parse().unwrap()) };
            match t[0] {
                "region" => {
                    l.w = n(1);
                    l.h = n(2);
                }
                "zlevels" => {
                    l.zlo = n(1);
                    l.zhi = n(2);
                }
                "clock" => l.clock = n(1),
                "ter" if t[1] == "all" => l.default_ter = t[2].into(),
                "ter" | "furn" | "item" => {
                    let c = Cell { x: n(1), y: n(2), z: z(4), id: t[3].into() };
                    match t[0] {
                        "ter" => l.ters.push(c),
                        "furn" => l.furns.push(c),
                        _ => l.items.push(c),
                    }
                }
                "monster" => l.mons.push(Cell { x: n(1), y: n(2), z: 0, id: t[3].into() }),
                "rad" => l.rads.push((n(1), n(2), z(4), n(3))),
                "field" => l.fields.push(FieldCell { x: n(1), y: n(2), z: z(6), id: t[3].into(), intensity: n(4), age: n(5) }),
                other => panic!("bad layout directive {other}"),
            }
        }
        l
    }

    pub fn load(path: &std::path::Path) -> Layout {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        Layout::parse(&name, &std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
    }
}
