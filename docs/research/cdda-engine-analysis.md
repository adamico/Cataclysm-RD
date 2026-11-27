# CDDA 0.H-RELEASE Engine analysis

Resolves [How modular is the CDDA 0.H-RELEASE Engine?](https://github.com/adamico/Cataclysm-RD/issues/7). Scored with the [Engine scoring rubric](engine-scoring-rubric.md). Terms: [GLOSSARY.md](../../GLOSSARY.md) and `/codebase-design` (module, interface, depth, seam, adapter, leverage, locality).

## 1. Header

- **Baseline:** Cataclysm: Dark Days Ahead 0.H-RELEASE "Herbert", repo `CleverRaven/Cataclysm-DDA`.
- **Commit:** `08f04fd07f028219a65f2f1fc8e73719dc9c954e`. The `0.H-RELEASE` tag resolves to exactly this commit, so tag and commit agree.
- **Date of analysis:** 2026-11-27.
- **Method:** static only. A shallow, read-only clone. No build, no `compile_commands.json`, no git history. The 2023–24 performance work is identified from code and comments only. `cloc` is not installed, so lines are `wc -l` physical lines (the count of `\n`). The script also reports non-blank lines and a comment-stripped "code" count that approximates cloc.
- **Clone:**
  ```sh
  git clone --depth 1 --branch 0.H-RELEASE https://github.com/CleverRaven/Cataclysm-DDA.git $SCRATCH/cdda
  git -C $SCRATCH/cdda rev-parse HEAD   # 08f04fd07f028219a65f2f1fc8e73719dc9c954e
  ```
- **Measurement commands** (Python 3.14, scripts kept outside the clone in `$SCRATCH/cdda-scripts/`, run with `-I`; full source in the collapsible blocks below):
  ```sh
  python3 -I json_types.py  $SCRATCH/cdda > out/json_types.txt        # JSON type counts (data/json + data/mods)
  python3 -I content_ctx.py out/json_types.txt                       # type families -> seed contexts
  python3 -I analyze.py     $SCRATCH/cdda out                          # split, context assignment, evidence.txt, tests.tsv
  python3 -I xref.py        $SCRATCH/cdda out/assignment.tsv 'name=regex' ...   # per-context occurrence counts
  python3 -I iface.py       $SCRATCH/cdda character.h map.h ...       # public-member counts per header
  ```
  The `xref.py` patterns used in this report are:
  - **Cache invalidation:** `set_transparency_cache_dirty=\bset_transparency_cache_dirty\s*\(`, and the same form for `set_seen_cache_dirty`, `set_floor_cache_dirty`, `set_outside_cache_dirty`, `set_pathfinding_cache_dirty` and `invalidate_reachability_cache`.
  - **Visibility, pathfinding and planning:** `vision_transparency=vision_transparency_cache`, `route(=\.route\s*\(`, `for_each_reachable=for_each_reachable|find_reachable`.
  - **Fields:** `add_field=\badd_field\s*\(`, `get_field=\b(get_field|get_field_intensity|get_field_age|has_field_at|remove_field|mod_field_intensity|set_field_intensity)\s*\(`, `fd_ids=\bfd_[a-z_]+\b`.
  - **Vehicles:** `veh_at=\bveh_at\s*\(`, `vehicle_cache=\b(add_vehicle_to_cache|clear_vehicle_level_caches|reset_vehicles_sm_pos|rebuild_vehicle_level_caches|set_veh_cached_parts|veh_cached_parts)\b`.
  - **Weather, time and flags:** `weather_get=\bget_weather\(\)`, `time_point=\btime_point\b|\btime_duration\b`, `flag_ids=\b(json_flag_|flag_)[A-Z0-9_]+\b`.
  - **Hard-coded Content IDs:** `itype_id=static const itype_id \w+\(`, and the same form for `mtype_id`, `efftype_id`, `ter_str_id`, `furn_str_id`, `trait_id` and so on, plus `all_static_ids=static const \w+_id \w+\(`.
  - **Type checks on Creatures:** `is_npc_avatar=dynamic_cast<\s*(const\s+)?(npc|avatar|Character)\s*\*|is_avatar\(\)|is_npc\(\)` and `is_monster_switch=dynamic_cast<\s*(const\s+)?monster\s*\*|is_monster\(\)`. A few one-off `grep`/`sed` reads (for example the list of `game::` functions, the `savegame_json.cpp` serializers, and reading `monster::plan`) are quoted inline where they are used.
- **Heuristics to know before comparing numbers:**
  - The *public interface* count is a regex parser. It tracks `class`/`struct`/`namespace` braces and `public:`/`private:`, and counts function-shaped and data-member lines in public scope. It is good for comparing contexts, not for exact totals.
  - *Global access* counts are textual occurrences of the rubric's accessors plus `get_player_view()`, `overmap_buffer.`, `get_creature_tracker()`, `get_event_bus()` and `get_timed_events()`. A static view can't tell reads from writes, so the "writes" row is only a regex for `g->x =` and `calendar::turn =`/`+=`.
  - *Include edges* are file-to-file `#include "..."` resolved by basename inside `src/`.

<details><summary><code>common.py</code> (split and context rules)</summary>

```python
"""Shared classification for the CDDA 0.H Engine analysis.

Split rule (rubric "Engine, Frontend and Content split"):
  Content  = everything under data/
  Vendored = src/third-party/**
  Frontend = src file that #includes a curses / SDL / ImGui header
             (regex FRONTEND_INCLUDE on the include target), or whose basename
             matches the rubric's name list (FRONTEND_NAME).
  Engine   = every other file under src/ (all extensions .cpp .h .hpp .inc)
Context rule (rubric "Assigning code to contexts"): see CONTEXT_RULES, applied to
the file stem (so foo.h and foo.cpp land together), first match wins; files left
unmatched go through one include-cluster pass (see contexts.py) and then to Unclaimed.
"""
import os, re

SRC_EXT = (".cpp", ".h", ".hpp", ".inc", ".c", ".cc")
FRONTEND_INCLUDE = re.compile(r'#\s*include\s*[<"]([^>"]*(curses|SDL|sdl_|imgui)[^>"]*)[>"]', re.I)
FRONTEND_NAME = re.compile(
    r'(_ui\.|^cata_tiles|^sdltiles|^output|menu|^ui_|^panels|^wincurse|^cursesport)')
INCLUDE = re.compile(r'^\s*#\s*include\s*"([^"]+)"', re.M)


def src_files(root):
    out = []
    for d, _, fs in os.walk(os.path.join(root, "src")):
        for f in fs:
            if f.endswith(SRC_EXT):
                out.append(os.path.relpath(os.path.join(d, f), root))
    return sorted(out)


def read(root, rel):
    with open(os.path.join(root, rel), encoding="utf-8", errors="replace") as fh:
        return fh.read()


def split_of(root, rel):
    """Literal rubric rule. Returns (split, reason)."""
    if rel.startswith("src/third-party/"):
        return "Vendored", "vendored dir"
    base = os.path.basename(rel)
    if FRONTEND_NAME.search(base):
        return "Frontend", "name"
    if FRONTEND_INCLUDE.search(read(root, rel)):
        return "Frontend", "include"
    return "Engine", "-"


# Adjusted rule (deviation, reported): a file that is Frontend ONLY because it includes
# cursesdef.h / sdl headers is re-admitted to the Engine (as "Ambiguous") when its stem
# matches a domain context rule or is one of the game-loop files below, except the
# display-only files in READMIT_EXCLUDE.
READMIT_LOOP = re.compile(r'^(game|do_turn|init)$')
READMIT_EXCLUDE = re.compile(r'^(player_display|vehicle_display|newcharacter|magic_teleporter_list|'
                             r'crafting_gui|iuse_software_.*|dialogue_win|veh_interact)$')
UI_TOKENS = re.compile(r'\b(uilist|ui_adaptor|catacurses::window|mvwprintz|mvwprintw|wnoutrefresh|'
                       r'draw_border|query_yn|popup\s*\(|string_input_popup|fold_and_print|wprintz|werase)\b')


def stem(rel):
    b = os.path.basename(rel)
    return b.rsplit(".", 1)[0]


def code_lines(text):
    """Non-blank lines after stripping // and /* */ comments (cloc-like approximation)."""
    text = re.sub(r'/\*.*?\*/', lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)
    n = 0
    for line in text.split("\n"):
        line = re.sub(r'//.*', '', line).strip()
        if line:
            n += 1
    return n


def phys_lines(text):
    return text.count("\n")  # wc -l semantics


def nonblank(text):
    return sum(1 for l in text.split("\n") if l.strip())


# Ordered (context, regex on stem). First match wins.
CONTEXT_RULES = [
    # --- Unclaimed first: cross-cutting scripting (EOC, conditions, talkers), events and
    #     telemetry, coordinate systems; these would otherwise be pulled into Creatures / Map
    ("Unclaimed", r'^(effect_on_condition|condition$|talker|event|achievement|stats_tracker|'
                  r'memorial_logger|diary$|kill_tracker|coordinates|coordinate_conversions|'
                  r'cube_direction|mapsharing|messages$)'),
    # --- Vehicles: vehicle*, veh_*, vpart*, turret, tileray (vehicle facing), grab (vehicle grab)
    ("Vehicles", r'^(vehicle|veh_|vpart|turret$|tileray$|grab$)'),
    # --- Fields: field*, map_field, fire, emit, scent, fungal_effects
    ("Fields", r'^(field|map_field$|fire$|emit$|scent_|fungal_effects$)'),
    # --- Time & Weather
    ("Time & Weather", r'^(calendar|weather|timed_event$)'),
    # --- Terrain & Map: map*, submap, mapgen*, overmap*, omdata, city, lighting/vision caches,
    #     traps, construction, examine, gates, pathfinding, regional settings, map extras
    ("Terrain & Map", r'^(map(?!_item_stack)|submap|mapgen|overmap|omdata|city$|lightmap|shadowcasting|'
                      r'level_cache|reachability_cache|trap|construction|iexamine|gates$|pathfinding|'
                      r'simple_pathfinding|regional_settings|start_location|cellular_automata|'
                      r'jmapgen_flags|magic_ter_fur|magic_ter_furn|maptile_fwd|cube_direction|'
                      r'clzones|explosion$)'),
    # --- Items
    ("Items", r'^(item|itype|iuse|inventory$|visitable|ammo|material|fault|relic|clothing_mod|'
              r'gun_mode|recipe|craft|requirements|temp_crafting|build_reqs|pocket_type|'
              r'active_item_cache|contents_change_handler|handle_liquid|flag$|item_|'
              r'damage$|pickup$)'),
    # --- Creatures: creature, character*, avatar*, npc*, monster/mon*, mtype, mattack*, body,
    #     mutation, bionics, effects, morale, martial arts, magic, skills, activities, behaviour
    ("Creatures", r'^(creature|character|avatar|npc|monster|mon|mtype|mattack|mutation|bionics|bodypart|'
                  r'body_part_set|subbodypart|anatomy|effect$|effect_source|morale|martialarts|melee|'
                  r'magic(?!_ter)|proficiency|skill|addiction|stomach|vitamin|suffer|player_|'
                  r'behavior|move_mode|weakpoint|disease|activity|climbing|speech|char_validity|'
                  r'trait_group|profession|bonuses|kill_tracker|mood_face|speed_description|'
                  r'mission|faction|basecamp|dialogue|npctalk|npctrade|shop_cons|scenario$|newcharacter|sounds$|'
                  r'harvest$|butchery_requirements|teleport$|consumption$|ranged$|projectile$|ballistics$|'
                  r'dispersion$)'),
]

# Unclaimed by design (infra): explicit list so the include-cluster pass can't pull them in.
UNCLAIMED_INFRA = re.compile(
    r'^(game$|do_turn|init$|main$|savegame|json|flexbuffer|generic_factory|string_id|int_id|type_id|'
    r'cata_|point|units|rng|debug|options|cached_options|filesystem|translation|translations|'
    r'catacharset|unicode|wcwidth|string_formatter|color|enum|enums|value_ptr|clone_ptr|colony|list$|'
    r'flat_set|hash_utils|mdarray|memory_fast|make_static|ordered_static|demangle|crash|compatibility|'
    r'platform_win|mingw|ofstream|io_tags|ret_val|try_parse|lru_cache|safe_reference|pimpl|'
    r'weighted_list|distribution|simplexnoise|line$|rect_range|cuboid|sets_intersect|cartesian|'
    r'math_|mmap_file|tgz_archiver|zzip|text_snippets|lang_stats|localized|system_locale|version|'
    r'get_version|prefix|mod_manager|mod_tracker|worldfactory|dependency_tree|path_info|'
    r'global_vars|common_types|all_enum_values|to_string_id|test_data|game_constants|shearing|'
    r'drawing_primitives|uistate|action$|handle_action|input$|gamemode|help$|past_games_info|'
    r'text_style|emscripten|auto_note|auto_pickup|distraction_manager|live_view|viewer|'
    r'iteminfo_query|rotatable_symbols|overlay_ordering|music|sdlsound|ascii_art|popup|'
    r'string_editor|string_input|wish|editmap|game_inventory|animation|computer|'
    r'mapsharing|pinyin|chkjson|display$|widget$|options_)')

# Adjusted rule, second half (deviation, reported): Engine-by-rule files that are display or
# input code by name and content move to Frontend.
FRONTEND_EXTRA = re.compile(
    r'^(cursesdef|advanced_inv.*|pixel_minimap_projectors|font_loader|mod_tileset|sdlsound|color|'
    r'color_loader|popup|uistate|display|widget|animation|drawing_primitives|editmap|game_inventory|'
    r'help|descriptions|bodygraph|veh_interact|crafting_gui|iuse_software_.*|dialogue_win|'
    r'magic_teleporter_list|auto_note|auto_pickup|distraction_manager|overlay_ordering|music|'
    r'string_editor_window|input|action|map_item_stack|item_search|iteminfo_query|live_view|'
    r'ascii_art|game_ui|rotatable_symbols|wish)$')


def pub_decls(t):
    stack = []
    fn = dm = 0
    t = re.sub(r'/\*.*?\*/', '', t, flags=re.S)
    pending = None
    for raw in t.split("\n"):
        l = re.sub(r'//.*', '', raw).strip()
        if not l or l.startswith("#"):
            continue
        m = re.match(r'(public|private|protected)\s*:', l)
        if m and stack:
            stack[-1] = (stack[-1][0], m.group(1)); continue
        k = None
        if re.match(r'(template\s*<.*>\s*)?(class|struct|union)\s+[\w:]+[^;]*$', l) or \
           re.match(r'(template\s*<.*>\s*)?(class|struct|union)\s+[\w:]+.*\{', l):
            k = "struct" if re.search(r'\b(struct|union)\b', l) else "class"
            pending = k
        elif re.match(r'namespace\b', l):
            pending = "ns"
        elif re.match(r'(enum)\b', l):
            pending = "enum"
        scope_ok = all(kk in ("ns", "class", "struct") and a == "public" for kk, a in stack)
        if scope_ok and pending not in ("ns", "enum", "struct", "class"):
            if not re.match(r'(return|if|while|for|typedef|using|static_assert|friend|template\s*<[^>]*>\s*$|\}|\{)', l):
                if re.search(r'[~\w:>\]]\s*\(', l) and (l.endswith(";") or l.endswith("{") or l.endswith(")") or l.endswith("const")):
                    fn += 1
                elif l.endswith(";") and "(" not in l and stack and stack[-1][0] in ("class", "struct"):
                    dm += 1
        for ch in l:
            if ch == "{":
                if pending in ("class", "struct"):
                    stack.append((pending, "private" if pending == "class" else "public"))
                elif pending == "ns":
                    stack.append(("ns", "public"))
                elif pending == "enum":
                    stack.append(("enum", "public"))
                else:
                    stack.append(("body", "-"))
                pending = None
            elif ch == "}":
                if stack:
                    stack.pop()
            elif ch == ";" and pending in ("class", "struct", "enum"):
                pending = None  # forward declaration
    return fn, dm
```
</details>

<details><summary><code>analyze.py</code></summary>

```python
"""CDDA 0.H Engine analysis: stats, context assignment and per-context evidence counts.

Usage: python3 -I analyze.py <clone-root> <out-dir>
Writes: <out-dir>/assignment.tsv, stats.txt, evidence.txt
All counts are static, regex-based; see common.py for the split and context rules.
"""
import os, re, sys, collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import (src_files, read, split_of, stem, code_lines, phys_lines, nonblank,
                    CONTEXT_RULES, UNCLAIMED_INFRA, INCLUDE, READMIT_LOOP, READMIT_EXCLUDE, UI_TOKENS, FRONTEND_EXTRA, pub_decls)

root, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
CTX = [c for c, _ in CONTEXT_RULES]
SEED = ["Creatures", "Terrain & Map", "Fields", "Items", "Vehicles", "Time & Weather", "Unclaimed"]

files = src_files(root)
text = {f: read(root, f) for f in files}
literal = {}
reason = {}
for f in files:
    literal[f], reason[f] = split_of(root, f)
# adjusted split
split = dict(literal)
ambiguous = []
for f in files:
    s = stem(f)
    if literal[f] == "Frontend" and reason[f] == "include" and not READMIT_EXCLUDE.search(s) and \
       (READMIT_LOOP.search(s) or any(re.search(rx, s) for _, rx in CONTEXT_RULES)):
        split[f] = "Engine"; ambiguous.append(f)
moved_fe = []
for f in files:
    if split[f] == "Engine" and FRONTEND_EXTRA.search(stem(f)):
        split[f] = "Frontend"; moved_fe.append(f)
by_base = {os.path.basename(f): f for f in files if not f.startswith("src/third-party/")}

# ---------- stats ----------
def tally(sp):
    st = collections.defaultdict(lambda: [0, 0, 0, 0])
    for f in files:
        s = st[sp[f]]
        s[0] += 1; s[1] += phys_lines(text[f]); s[2] += nonblank(text[f]); s[3] += code_lines(text[f])
    return st
st_lit, st = tally(literal), tally(split)
content = [0, 0, 0]; content_json = [0, 0]; content_bin = 0
for d, _, fs in os.walk(os.path.join(root, "data")):
    for fn in fs:
        pth = os.path.join(d, fn)
        content[0] += 1
        b = open(pth, "rb").read()
        if b"\0" in b[:8192]:
            content_bin += 1; continue
        t = b.decode("utf-8", "replace")
        n = phys_lines(t); content[1] += n
        content[2] += nonblank(t)
        if fn.endswith(".json"):
            content_json[0] += 1; content_json[1] += n

# ---------- include graph (file level) ----------
inc = {f: [by_base[i] for i in INCLUDE.findall(text[f]) if i in by_base and by_base[i] != f]
       for f in files if split[f] != "Vendored"}
incby = collections.defaultdict(set)
for f, ts in inc.items():
    for t in ts:
        incby[t].add(f)

# ---------- context assignment ----------
ctx = {}
how = {}
eng = [f for f in files if split[f] == "Engine"]
for f in eng:
    s = stem(f)
    for c, rx in CONTEXT_RULES:
        if re.search(rx, s):
            ctx[f] = c; how[f] = "name/anchor"; break
    else:
        if UNCLAIMED_INFRA.search(s):
            ctx[f] = "Unclaimed"; how[f] = "infra-list"
# include-cluster pass for leftovers (grouped by stem)
left = collections.defaultdict(list)
for f in eng:
    if f not in ctx:
        left[stem(f)].append(f)
for s, fs in sorted(left.items()):
    cnt = collections.Counter()
    for f in fs:
        for t in list(inc.get(f, [])) + list(incby.get(f, [])):
            if t in ctx and ctx[t] != "Unclaimed":
                cnt[ctx[t]] += 1
    tot = sum(cnt.values())
    if tot >= 3 and cnt.most_common(1)[0][1] / tot >= 0.5:
        c = cnt.most_common(1)[0][0]; h = f"include-cluster {cnt.most_common(1)[0][1]}/{tot}"
    else:
        c = "Unclaimed"; h = f"leftover ({dict(cnt)})"
    for f in fs:
        ctx[f] = c; how[f] = h

with open(os.path.join(out, "assignment.tsv"), "w") as fh:
    for f in files:
        fh.write(f"{f}\t{literal[f]}:{reason[f]}\t{split[f]}\t{ctx.get(f, '-')}\t{how.get(f, '-')}\t{phys_lines(text[f])}\n")

def group(f):
    return ctx.get(f, split[f])  # Frontend / Vendored outside the Engine

# ---------- evidence ----------
E = []
def p(*a):
    E.append(" ".join(str(x) for x in a))

per = collections.defaultdict(lambda: [0, 0, 0])
for f in eng:
    r = per[ctx[f]]; r[0] += 1; r[1] += phys_lines(text[f]); r[2] += code_lines(text[f])
p("## Engine per context: files, physical lines, code lines")
for c in SEED:
    p(c, *per[c])

# include edges between groups (file->file), both directions
edges = collections.Counter()
for f, ts in inc.items():
    for t in ts:
        a, b = group(f), group(t)
        if a != b:
            edges[(a, b)] += 1
G = SEED + ["Frontend"]
p("\n## Include edges (row includes column), file-level")
p("from\\to | " + " | ".join(G))
for a in G:
    p(a, "|", " | ".join(str(edges[(a, b)]) for b in G))

# fan-in / fan-out by include, per context (files)
p("\n## Include fan-in (outside files including a context header) / fan-out (outside headers included)")
for c in SEED:
    fin = {f for f, ts in inc.items() if group(f) != c and any(ctx.get(t) == c for t in ts)}
    fout = {t for f, ts in inc.items() if ctx.get(f) == c for t in ts if group(t) != c}
    p(c, "fan-in-files", len(fin), "fan-out-headers", len(fout))

# global access
GLOB = {
    "g->": r'\bg->', "get_map()": r'\bget_map\(\)', "get_player_character()": r'\bget_player_character\(\)',
    "get_avatar()": r'\bget_avatar\(\)', "get_weather()": r'\bget_weather\(\)',
    "calendar::turn": r'\bcalendar::turn\b', "get_player_view()": r'\bget_player_view\(\)',
    "overmap_buffer.": r'\bovermap_buffer\s*\.', "get_creature_tracker()": r'\bget_creature_tracker\(\)',
    "get_event_bus()": r'\bget_event_bus\(\)', "get_timed_events()": r'\bget_timed_events\(\)',
}
p("\n## Global access per context (occurrences; per kLOC code in brackets)")
p("context | " + " | ".join(GLOB) + " | total | per-kLOC")
for c in SEED:
    fs = [f for f in eng if ctx[f] == c]
    row = [sum(len(re.findall(rx, text[f])) for f in fs) for rx in GLOB.values()]
    tot = sum(row)
    p(c, "|", " | ".join(map(str, row)), "|", tot, "|", round(tot / max(per[c][2], 1) * 1000, 1))
# writes heuristic: assignment to a g-> member or calendar::turn
p("\n## Global write heuristic: `g->x = ` / `g->x += ` and `calendar::turn =|+=`")
for c in SEED:
    fs = [f for f in eng if ctx[f] == c]
    w = sum(len(re.findall(r'\bg->\w+\s*(\+|-)?=[^=]', text[f])) for f in fs)
    w2 = sum(len(re.findall(r'\bcalendar::turn\s*(\+|-)?=[^=]', text[f])) for f in fs)
    p(c, "g-writes", w, "turn-writes", w2)
# static mutable data at namespace/file scope (heuristic: lines starting with `static ` not const/constexpr, not functions)
p("\n## File-scope `static` non-const data definitions (heuristic, column-0 `static` lines without `(`... `)` function shape)")
for c in SEED:
    fs = [f for f in eng if ctx[f] == c]
    n = 0
    for f in fs:
        for l in text[f].split("\n"):
            if re.match(r'^static\s+(?!const\b|constexpr\b|inline\b|void\b|bool\b\s+\w+\s*\()', l) and \
               not re.search(r'\)\s*(const)?\s*(\{|;|$)', l) and "(" not in l.split("=")[0]:
                n += 1
    p(c, n)

# public interface size heuristic in headers
p("\n## Public interface size heuristic (context headers): public functions, public data members, headers")
for c in SEED:
    hs = [f for f in eng if ctx[f] == c and f.endswith((".h", ".hpp"))]
    a = b = 0
    for h in hs:
        x, y = pub_decls(text[h]); a += x; b += y
    p(c, "pub-fn", a, "pub-data", b, "headers", len(hs))

# class -> context map from headers; friend and inheritance crossing contexts
cls = {}
for f in files:
    if split[f] == "Vendored" or not f.endswith((".h", ".hpp")):
        continue
    for m in re.finditer(r'^\s*(?:class|struct)\s+(?:\w+\s+)?(\w+)\s*(?:final\s*)?(?::[^;{]*)?\{', text[f], re.M):
        cls.setdefault(m.group(1), group(f))
p("\n## friend declarations crossing contexts (declaring context -> friend's context)")
fr = collections.Counter(); frl = collections.defaultdict(list)
for f in files:
    if split[f] == "Vendored":
        continue
    for m in re.finditer(r'\bfriend\s+(?:class\s+|struct\s+)?([\w:]+)', text[f]):
        n = m.group(1).split("::")[-1]
        if n in cls and cls[n] != group(f):
            fr[(group(f), cls[n])] += 1; frl[group(f)].append(f"{os.path.basename(f)}:{n}")
for (a, b), n in sorted(fr.items()):
    p(a, "->", b, n)
for a, l in frl.items():
    p(" ", a, ":", ", ".join(sorted(set(l))))
p("\n## Inheritance crossing contexts (derived context <- base context: examples)")
ih = collections.defaultdict(list)
for f in files:
    if split[f] == "Vendored":
        continue
    for m in re.finditer(r'^\s*(?:class|struct)\s+(\w+)\s*(?:final\s*)?:\s*([^{;]+)\{', text[f], re.M):
        for base in re.findall(r'(?:public|protected|private)?\s*([\w:]+)(?:<[^>]*>)?\s*(?:,|$)', m.group(2)):
            if base.startswith("std::"):
                continue
            b = base.split("::")[-1]
            if b in cls and cls[b] != group(f) and b not in ("enable_shared_from_this",):
                ih[(group(f), cls[b])].append(f"{m.group(1)}:{b}")
for k, v in sorted(ih.items()):
    p(k[0], "<-", k[1], len(v), ", ".join(sorted(set(v))[:12]))

# tests per context
p("\n## Tests per context (tests/*.cpp by stem rule) and fixture calls per file")
tdir = os.path.join(root, "tests")
tc = collections.defaultdict(list)
FIX = r'\b(clear_map|clear_avatar|clear_character|clear_creatures|clear_npcs|clear_vehicles|build_test_map|'\
      r'set_time|put_player_at|get_map\(\)|get_player_character\(\)|get_avatar\(\)|clear_map_and_put_player_underground)\b'
for fn in sorted(os.listdir(tdir)):
    if not fn.endswith(".cpp"):
        continue
    s = re.sub(r'(_tests?|_test_.*)$', '', fn[:-4])
    t = open(os.path.join(tdir, fn), encoding="utf-8", errors="replace").read()
    # rule: name rule on the stem first (Unclaimed name rule skipped); else majority context of
    # the Engine src headers it includes (Unclaimed headers ignored) if >= 40%; else Unclaimed
    c = None
    for cc, rx in CONTEXT_RULES:
        if cc != "Unclaimed" and re.search(rx, s):
            c = cc; break
    if c is None:
        cnt = collections.Counter(ctx[by_base[i]] for i in INCLUDE.findall(t)
                                  if i in by_base and by_base[i] in ctx and ctx[by_base[i]] != "Unclaimed")
        tot = sum(cnt.values())
        c = cnt.most_common(1)[0][0] if tot and cnt.most_common(1)[0][1] / tot >= 0.4 else "Unclaimed"
    tc[c].append((fn, len(re.findall(FIX, t)), len(re.findall(r'\bTEST_CASE\s*\(', t))))
for c in SEED:
    l = tc[c]
    nfix = sum(x[1] for x in l); ncase = sum(x[2] for x in l)
    nz = sum(1 for x in l if x[1] == 0)
    p(c, "files", len(l), "TEST_CASEs", ncase, "fixture-calls", nfix, "files-without-fixture", nz)
with open(os.path.join(out, "tests.tsv"), "w") as fh:
    for c in SEED:
        for fn, a, b in tc[c]:
            fh.write(f"{c}\t{fn}\t{a}\t{b}\n")

with open(os.path.join(out, "stats.txt"), "w") as fh:
    for lab, S in (("LITERAL", st_lit), ("ADJUSTED", st)):
        for k in ("Engine", "Frontend", "Vendored"):
            fh.write(f"{lab} {k}\tfiles {S[k][0]}\tphys {S[k][1]}\tnonblank {S[k][2]}\tcode {S[k][3]}\n")
    fh.write(f"Content\tfiles {content[0]}\tbinary {content_bin}\tphys(text) {content[1]}\tnonblank {content[2]}\n")
    uiheavy = sorted(((len(UI_TOKENS.findall(text[f])), f) for f in files
                      if literal[f] == "Engine"), reverse=True)
    fh.write("ENGINE-BY-RULE WITH UI TOKENS >= 20: " + " ".join(
        f"{os.path.basename(f)}({n})" for n, f in uiheavy if n >= 20) + "\n")
    fh.write("UI TOKENS IN RE-ADMITTED: " + " ".join(
        f"{os.path.basename(f)}({len(UI_TOKENS.findall(text[f]))})" for f in ambiguous) + "\n")
    fh.write("MOVED TO FRONTEND (adjusted): " + " ".join(f"{os.path.basename(f)}({phys_lines(text[f])})" for f in moved_fe) + "\n")
    fh.write("AMBIGUOUS (re-admitted): " + " ".join(f"{os.path.basename(f)}({phys_lines(text[f])})" for f in ambiguous) + "\n")
    fh.write(f"Content-json\tfiles {content_json[0]}\tphys {content_json[1]}\n")
with open(os.path.join(out, "evidence.txt"), "w") as fh:
    fh.write("\n".join(E) + "\n")
print("ok")
```
</details>

<details><summary><code>json_types.py</code>, <code>content_ctx.py</code>, <code>xref.py</code>, <code>iface.py</code></summary>

```python
# Usage: python3 -I json_types.py <clone>
# Prints counts of JSON "type" values under data/json and data/mods (one object = one count).
import json, os, sys, collections
root = sys.argv[1]
c = collections.Counter()
for base in ("data/json", "data/mods"):
    for d, _, fs in os.walk(os.path.join(root, base)):
        for f in fs:
            if not f.endswith(".json"):
                continue
            try:
                data = json.load(open(os.path.join(d, f), encoding="utf-8"))
            except Exception:
                continue
            if isinstance(data, dict):
                data = [data]
            if not isinstance(data, list):
                continue
            for o in data:
                if isinstance(o, dict) and isinstance(o.get("type"), str):
                    c[o["type"]] += 1
for t, n in sorted(c.items(), key=lambda x: -x[1]):
    print(f"{n}\t{t}")
```

```python
"""Assign JSON type families (json_types.txt from json_types.py) to seed contexts.
Usage: python3 -I content_ctx.py json_types.txt
Unlisted types fall to Unclaimed (printed so the list can be audited).
"""
import sys, collections
M = {
 "Creatures": "MONSTER SPECIES monstergroup MONSTER_FACTION monster_attack monster_flag monster_adjustment "
   "MONSTER_BLACKLIST MONSTER_WHITELIST mutation mutation_type mutation_category trait_group TRAIT_MIGRATION "
   "TRAIT_BLACKLIST bionic bionic_migration effect_type SPELL enchantment martial_art technique "
   "weapon_category body_part sub_body_part anatomy body_graph limb_score character_mod weakpoint_set "
   "skill skill_display_type skill_boost proficiency proficiency_category profession profession_group "
   "profession_item_substitutions npc npc_class speech talk_topic faction mission_definition "
   "morale_type addiction_type vitamin disease_type movement_mode behavior activity_type mood_face "
   "speed_description dream climbing_aid harvest harvest_drop_type butchery_requirement hit_range "
   "shopkeeper_consumption_rates shopkeeper_blacklist scenario SCENARIO_BLACKLIST practice",
 "Terrain & Map": "terrain furniture mapgen palette overmap_terrain overmap_special overmap_special_migration "
   "city_building city overmap_location overmap_connection overmap_land_use_code oter_id_migration "
   "map_extra region_settings region_overlay trap construction construction_group construction_category "
   "gate ter_furn_transform connect_group start_location LOOT_ZONE",
 "Fields": "field_type emit scent_type",
 "Items": "ARMOR GENERIC COMESTIBLE TOOL BOOK AMMO GUN MAGAZINE TOOL_ARMOR GUNMOD BIONIC_ITEM PET_ARMOR "
   "ENGINE WHEEL TOOLMOD BATTERY item_group MIGRATION ITEM_CATEGORY ITEM_BLACKLIST item_action material "
   "ammunition_type ammo_effect tool_quality fault fault_fix relic_procgen_data clothing_mod recipe "
   "uncraft recipe_group recipe_category nested_category requirement json_flag damage_type "
   "damage_info_order charge_removal_blacklist temperature_removal_blacklist",
 "Vehicles": "vehicle vehicle_part vehicle_part_category vehicle_part_migration vehicle_group "
   "vehicle_placement vehicle_spawn",
 "Time & Weather": "weather_type",
}
t2c = {t: c for c, ts in M.items() for t in ts.split()}
agg = collections.Counter(); fam = collections.Counter(); left = []
for line in open(sys.argv[1]):
    n, t = line.rstrip("\n").split("\t")
    c = t2c.get(t, "Unclaimed")
    agg[c] += int(n); fam[c] += 1
    if c == "Unclaimed":
        left.append(f"{t}:{n}")
for c in ["Creatures", "Terrain & Map", "Fields", "Items", "Vehicles", "Time & Weather", "Unclaimed"]:
    print(c, "families", fam[c], "objects", agg[c])
print("Unclaimed types:", " ".join(left))
```

```python
"""Cross-reference counter: for each regex, count occurrences per context of the calling file.

Usage: python3 -I xref.py <clone> <assignment.tsv> <name=regex>...
Context of a file comes from assignment.tsv (column 4 for Engine files, else the split).
Prints: name | occurrences per context (and files per context in brackets).
"""
import os, re, sys, collections
root, asg = sys.argv[1], sys.argv[2]
ctx = {}
for line in open(asg):
    f, lit, split, c, how, n = line.rstrip("\n").split("\t")
    if split != "Vendored":
        ctx[f] = c if split == "Engine" else split
ORDER = ["Creatures", "Terrain & Map", "Fields", "Items", "Vehicles", "Time & Weather", "Unclaimed", "Frontend"]
text = {f: open(os.path.join(root, f), encoding="utf-8", errors="replace").read() for f in ctx}
print("name | " + " | ".join(ORDER))
for spec in sys.argv[3:]:
    name, rx = spec.split("=", 1)
    r = re.compile(rx)
    occ = collections.Counter(); fl = collections.Counter()
    for f, t in text.items():
        n = len(r.findall(t))
        if n:
            occ[ctx[f]] += n; fl[ctx[f]] += 1
    print(name, "|", " | ".join(f"{occ[c]}({fl[c]})" for c in ORDER))
```

```python
"""Usage: python3 -I iface.py <clone> header...  -> public fn / data member counts per header (heuristic)."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import pub_decls, read
for h in sys.argv[2:]:
    print(h, *pub_decls(read(sys.argv[1], "src/" + h)))
```
</details>

## 2. Code statistics

The main table uses the **literal rubric rule**, so it compares directly with the Bright Nights report:

| Split | Files | Lines (`wc -l`) | Non-blank | Code (comment-stripped) |
|---|---|---|---|---|
| Engine | 668 | 348,849 | 309,735 | 274,053 |
| Frontend | 130 | 177,374 | 158,355 | 144,426 |
| Content | 6,561 (1,353 binary; 5,107 `.json`) | 1,515,178 text lines (1,498,470 in `.json`) | 1,512,999 | n/a |
| Vendored (excluded): `src/third-party/` (flatbuffers, ghc, pinyin) | 26 | 21,577 | 19,486 | 16,843 |

Content holds 72,488 JSON objects across 157 `type` values.

**Why the literal rule misleads here.** 80 Engine files include `cursesdef.h` just to get `catacurses::window` for one popup or draw call. The literal rule sends those files to Frontend, and among them are the biggest Engine files in the game: `game.cpp` (13,699 lines), `character.cpp` (13,136), `map.cpp` (10,427), `iexamine.cpp`, `monster.cpp`, `npc.cpp`, `creature.cpp`, `ranged.cpp` and `magic.cpp`. On this rule, 51% of "Frontend" is Simulation code. So the per-context scoring uses an **adjusted rule** (a deviation, applied mechanically by `common.py`):

1. **Re-admit to the Engine, as Ambiguous:** a file that is Frontend *only* because of a curses/SDL include, when its stem matches a domain context rule or is `game`, `do_turn` or `init`. Display-only files (`player_display`, `vehicle_display`, `newcharacter`, `magic_teleporter_list`, `crafting_gui`, `iuse_software_*`, `dialogue_win`, `veh_interact`) stay out. This gives 25 files, 90,891 lines.
2. **Move to Frontend:** 58 Engine-by-rule files that are display or input code by name and content: `action`, `input`, `advanced_inv*`, `cursesdef.h`, `color`, `popup`, `display`, `widget`, `editmap`, `veh_interact.cpp`, `game_inventory.h`, `uistate.h`, `item_search`, `iteminfo_query`, `map_item_stack`, `music`, `ascii_art` and others (the full list is `FRONTEND_EXTRA` in `common.py`).

| Split (adjusted, used for scoring) | Files | Lines | Code |
|---|---|---|---|
| Engine | 636 | 421,708 | 335,351 |
| Frontend | 162 | 104,515 | 83,128 |

**Ambiguous files** (Engine by the adjusted rule, but mixing in UI; UI-token counts from `UI_TOKENS` in brackets): `game.cpp` (233), `game.h` (27), `iexamine.cpp` (100), `faction_camp.cpp` (56), `faction.cpp` (47), `ranged.cpp` (36), `item_contents.cpp` (31), `messages.cpp` (22), `mission_companion.cpp` (21), `construction.cpp` (20), `magic.cpp` (19), `character.cpp` (14), `avatar.cpp`, `clzones.cpp`, `monster.cpp` and `npc.cpp` (10 each), plus `creature.cpp`, `do_turn.cpp`, `init.cpp`, `map.cpp`, `martialarts.cpp`, `morale.cpp`, `pickup.cpp`, `scent_map.cpp` and `sounds.cpp` (at most 8 each). The literal-Engine files with heavy UI are `iuse.cpp` (65), `veh_appliance.cpp` (24) and `iuse_actor.cpp` (20).

The rubric's ImGui clause matches nothing: 0.H has no ImGui headers or files.

**Engine per context** (adjusted rule; `assignment.tsv` in the scripts output has every file):

| Context | Files | Lines | Code |
|---|---|---|---|
| Creatures | 198 | 162,348 | 131,126 |
| Terrain & Map | 81 | 67,722 | 53,642 |
| Fields | 15 | 4,474 | 3,546 |
| Items | 73 | 71,151 | 55,915 |
| Vehicles | 22 | 23,219 | 17,655 |
| Time & Weather | 10 | 4,134 | 3,049 |
| Unclaimed | 237 | 88,660 | 70,418 |

**Content per context** (`content_ctx.py`; JSON objects, with the number of `type` families in brackets):

| Context | Objects |
|---|---|
| Creatures | 17,095 (63) |
| Terrain & Map | 20,059 (25) |
| Fields | 203 (3: `field_type`, `emit`, `scent_type`) |
| Items | 29,388 (39) |
| Vehicles | 941 (7) |
| Time & Weather | 24 (1: `weather_type`) |
| Unclaimed | 4,778 (19; of which `effect_on_condition` is 2,002 and `widget` 1,047) |

**Dependency map:** file-level `#include` edges. Each cell counts the edges from the row's files to the column's headers.

| from \ to | Crea | T&M | Fields | Items | Veh | T&W | Uncl | Front |
|---|---|---|---|---|---|---|---|---|
| Creatures | – | 212 | 36 | 315 | 72 | 86 | 1210 | 167 |
| Terrain & Map | 118 | – | 14 | 71 | 44 | 25 | 506 | 54 |
| Fields | 18 | 11 | – | 6 | 2 | 8 | 56 | 5 |
| Items | 135 | 60 | 8 | – | 32 | 31 | 423 | 84 |
| Vehicles | 50 | 33 | 1 | 50 | – | 8 | 133 | 25 |
| Time & Weather | 16 | 14 | 0 | 3 | 0 | – | 73 | 3 |
| Unclaimed | 247 | 81 | 15 | 79 | 22 | 33 | – | 102 |
| Frontend | 234 | 101 | 10 | 117 | 52 | 40 | 816 | – |

**Include fan-in and fan-out:**

| Context | Fan-in (outside files that include one of its headers) | Fan-out (outside headers it includes) |
|---|---|---|
| Creatures | 156 | 180 |
| Terrain & Map | 147 | 163 |
| Fields | 51 | 54 |
| Items | 178 | 156 |
| Vehicles | 84 | 109 |
| Time & Weather | 167 | 64 |
| Unclaimed | 487 | 183 |

Every domain pair has edges in both directions. The Engine has no layering between contexts.

**Context assignment.**

- **Method:** an ordered stem-regex table in `common.py`. Its families were taken from the 157 JSON types and their loader registrations in `init.cpp` (`DynamicDataLoader::initialize`, lines 248–493). An explicit infrastructure list goes to Unclaimed. One include-cluster pass handles leftovers: a file joins a context if that context holds at least 50% of at least 3 edges.
- **Assigned by include cluster:** `flood_fill.h` and `fragment_cloud.h` (to Terrain & Map).
- **Left to Unclaimed:** `assign.*`, a generic JSON-assign helper with no dominant cluster.
- **Doubtful assignments:**
  - `explosion.cpp` and `clzones.*` went to Terrain & Map.
  - `harvest`, `butchery_requirements` and `teleport` went to Creatures.
  - The combat files (`ranged`, `ballistics`, `projectile`, `dispersion`) went to Creatures, while `damage` went to Items.
  - The NPC-society files (`mission*`, `faction*`, `basecamp`, `faction_camp`, `npctalk*`, `dialogue*`) went to Creatures.
  - `sounds.cpp` (noise propagation plus SFX) went to Creatures.
  - `pickup.cpp` went to Items.
  - The scripting files (`effect_on_condition`, `condition`, `talker*`, `math_parser*`) and the event and telemetry files (`event*`, `achievement`, `stats_tracker`, `memorial_logger`, `kill_tracker`, `diary`, `messages`) went to Unclaimed.
  - The coordinate systems (`coordinates`, `coordinate_conversions`, `cube_direction`) went to Unclaimed.

## 3. Per-context scorecards

Abbreviations used in the evidence:
- **glob** is the total of global accessors and **/kLOC** is that total per thousand comment-stripped code lines.
- **pub-fn** and **pub-data** are the public functions and public data members in the context's headers.
- **static IDs** counts lines of the form `static const <x>_id name( "..." )`, which hard-code a Content ID in C++.

### Creatures

- **Files:** 198 files, 162,348 lines.
- **Main concepts:** `Creature` (`creature.h`, 176 `virtual`s); `Character`, split over `character.cpp` and 13 `character_*.cpp` files; `avatar`; `npc`; `monster`, with `monmove.cpp`; `mtype`; `effect`/`effect_type`; `mutation_branch`; `bionic_data`; `spell_type`; martial arts; proficiency; skill; morale; activities; behaviour trees; `creature_tracker`; and NPC dialogue, missions, factions and basecamps (doubtful, see section 2).

| Criterion | Score | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 3 | med | Each main concept has a home class (`Character`, `monster`, `npc`, `mtype`, `effect_type`, `mutation_branch`). But creature rules also live in `game.cpp`: of 258 `game::` functions, about 60 handle creatures, for example `knockback`, `fling_creature`, `swap_critters`, `place_critter_at`, `spawn_npc` and `despawn_nonlocal_monsters`. The per-turn monster driver, including the `bio_alarm` check, is the free function `monmove()` in `do_turn.cpp:226`. Field effects on creatures are `map::creature_in_field`/`monster_in_field`/`player_in_field` (`map_field.cpp`), and weather wetting is `wet_character` in `weather.h`. |
| M2 Language alignment | 3 | med | `MONSTER`→`mtype`, `effect_type`→`effect_type`, `SPELL`→`spell_type`, `bionic`→`bionic_data` and `npc`→`npc_template` line up. Secondary names drift: `mutation`→`mutation_branch`/`trait_id`, the legacy `player_` stems (`player_activity`, `player_hardcoded_effects`, `player_difficulty`) although the `player` class is gone, and `monster_attack`→`mattack_actor`. |
| M3 Context isolation | 1 | high | glob 2,110 (16.1/kLOC): `get_map()` 527, `calendar::turn` 338, `g->` 337, `get_player_character()` 312, `overmap_buffer.` 197, `get_creature_tracker()` 118. Other contexts reach in by type: `dynamic_cast<npc/avatar/Character*>`, `is_npc()` and `is_avatar()` appear 76 times in Items, 49 in Terrain & Map, 37 in Frontend and 26 in Unclaimed. `creature_tracker` is owned by `game` and declares `friend game` (`creature_tracker.h`). |
| Q1 Leverage | 2 | med | There are a few deep entry points: `monster::plan()`/`move()`, `Character::process_turn()` and `Creature::deal_damage()`. But they sit behind 5,309 pub-fn and 1,986 pub-data in 90 headers, and `character.h` alone has 1,097 public functions and 279 public data members (`iface.py`). |
| Q2 Interface size | 1 | high | Thousands of public members. Hidden ordering rules: `creature_tracker::invalidate_reachability_cache()` "must be called when persistent visibility from terrain or furniture changes" (comment in `creature_tracker.h`), and `map.cpp` calls it 4 times. `g->cleanup_dead()` brackets `monmove()`. The moncam cache must be diffed in `map::build_map_cache`. |
| Q3 Locality | 2 | med | 721 include edges go to sibling domain contexts and 1,210 to Unclaimed. 2,202 static IDs sit in 69 files; for comparison, Items has 464. Serializers live outside the context in `savegame_json.cpp`: `monster` 6, `npc` 4, `avatar` 4, `Character`/`Creature` 2 each, and more. A new monster behaviour typically touches `monmove.cpp`, `mtype`, and often `map_field.cpp` or `game.cpp`. |
| Q4 Seam cleanliness | 1 | high | Deep hierarchy: `Creature : viewer` (an Unclaimed base) has the subclasses `monster`, `Character : Creature, visitable` (an Items base), and `avatar`/`npc : Character`. Raw `Creature*`/`monster*` pointers are held across the context (`monster_plan::target`, `level_cache` users, `creature_tracker::creatures_by_zone_and_faction_`). Globals run in both directions. |
| Q5 Testability | 3 | med | 75 test files with 463 `TEST_CASE`s. There are 532 fixture calls (`clear_map`, `clear_avatar`, `get_map()`, `get_player_character()` and similar), and 57 of the 75 files use one. Tests go through public calls but need the global map and avatar. |

### Terrain & Map

- **Files:** 81 files, 67,722 lines. `map` (`map.cpp`, `map.h`), `submap`, `mapbuffer`, `mapdata` (`ter_t`/`furn_t`), `mapgen*`, `overmap*`/`omdata`, `lightmap`/`shadowcasting`/`level_cache`/`reachability_cache`, `pathfinding`, `trap*`, `construction*`, `iexamine*`, `gates`, `map_extras`, `regional_settings` and `clzones`.

| Criterion | Score | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 3 | med | Terrain, furniture, overmap terrain, mapgen and traps each have a home (`mapdata.h`, `omdata.h`, `mapgen.h`, `trap.h`). But `map` is also the home of other contexts' rules: 34 vehicle `map::` functions (`vehmove`, `vehproceed`, `displace_vehicle`, `vehicle_vehicle_collision`, ...), field processing as `map::` members, and item storage (39 item-operation declarations in `map.h`). Furniture interaction is a 6,920-line switchboard in `iexamine.cpp`. `game.cpp` holds `vertical_move`, `update_map` and `load_map`. |
| M2 Language alignment | 3 | med | `terrain`→`ter_t`, `furniture`→`furn_t`, `overmap_terrain`→`oter_t`/`oter_type_t`, `trap`→`trap`, `mapgen`/`palette`→`mapgen_palette`, `overmap_special`→`overmap_special`. Drift: the abbreviations `ter`, `furn` and `oter`, and `map` meaning the reality bubble, not the world. The typed coordinates (`tripoint_bub_ms`, `tripoint_abs_omt`) are a good shared language, but untyped `tripoint` overloads remain (`// TODO: fix point types`). |
| M3 Context isolation | 2 | med | glob 658 (12.3/kLOC): `get_map()` 242, `g->` 116, `calendar::turn` 88, `overmap_buffer.` 68. The map reads other contexts' globals inside its caches: `build_transparency_cache` calls `get_weather()`, and `build_map_cache` calls `get_avatar()` and keeps function-local `static` player state. Every other context mutates the map through the `get_map()` singleton. |
| Q1 Leverage | 3 | med | There are deep entry points (`map::route`, `build_map_cache`, `generate_lightmap`, mapgen `generate`, `overmapbuffer`) beside a very wide surface: `map.h` has 434 public functions and 107 public data members. |
| Q2 Interface size | 1 | high | 1,748 pub-fn and 749 pub-data. Hand-maintained cache invalidation is part of the interface: `set_transparency_cache_dirty` is called from Terrain & Map 14 times, Vehicles 8, Time & Weather 1 and Frontend 1. Other hand-called hooks are `set_seen_cache_dirty`, `set_floor_cache_dirty` (5 calls in Vehicles), `set_outside_cache_dirty`, `set_pathfinding_cache_dirty` and `invalidate_reachability_cache` (on another context's object). Ordering is spelled out in comments, for example "We need floor cache before checking falling" (`do_turn.cpp`). |
| Q3 Locality | 2 | med | `level_cache` holds vehicle caches (`vehicle_list`, `veh_cached_parts`) and the field cache. Terrain and furniture rules are also used through 535 static IDs in 19 Terrain & Map files and 81 `ter_str_id` IDs in 4 Creatures files. A cache or terrain rule change ripples into Vehicles, Fields and Creatures (`map::sees` users). |
| Q4 Seam cleanliness | 1 | high | `submap` owns other contexts' storage: `std::vector<std::unique_ptr<vehicle>> vehicles`, `cata::colony<item>` per tile, and fields. `level_cache` keeps `std::set<vehicle*>`. Inheritance crosses in from Items: `map_stack : item_stack`, `map_cursor : visitable`. `map.h` declares 11 `friend`s, including the Frontend's `editmap`. |
| Q5 Testability | 3 | med | 28 test files with 106 cases, 15 of them fixture-free. Pure algorithms test well, for example `shadowcasting_test` (16 cases), `submap_load_test` (14), `map_iterator_test` and `overmap_test`. The map itself needs `clear_map()` and the global map (`map_helpers.cpp`). |

### Fields

- **Files:** 15 files, 4,474 lines: `field.*`, `field_type.*`, `map_field.*`, `fire.h`, `emit.*`, `scent_map.*`, `scent_block.*`, `fungal_effects.*`.
- **Split from Terrain & Map:** field storage lives in `submap`, and the hooks into the caches are `map::on_field_modified` in `map.cpp:6540` and the translucency step of `build_transparency_cache` in `lightmap.cpp`.

| Criterion | Score | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 3 | med | `field_type`, `field_entry`, `field_intensity_level` and the processor pipeline have one home (`map_field.cpp:2164`, `processors_for_type`). But 13 field types are special-cased by ID there (`ft.id == fd_fire` and so on), and `fd_*` IDs appear 103 times in 19 Creatures files, 80 in 9 Terrain & Map files, 24 in Unclaimed and 19 in Items. Field effects on vision are computed in `lightmap.cpp`. |
| M2 Language alignment | 4 | med | `field_type` (JSON) maps to `field_type`, `intensity_levels` to `field_intensity_level`, and `emit` to `emit_id`. The hard-coded `fd_*` names are the JSON IDs, so the names line up almost one to one. |
| M3 Context isolation | 2 | med | glob is low (30, 8.5/kLOC). But processing is written as `map::` members over the global map: `process_fields_in_submap` calls `get_player_character()`, `overmap_buffer.ter()` and `get_scent()`. Other contexts call `add_field`/`get_field*` freely: 49 and 37 times from Creatures, 59 and 86 from Terrain & Map. |
| Q1 Leverage | 4 | med | One call, `map::process_fields()`, runs all field behaviour, and the data-driven processor list is selected once per type. The caller interface is small (`add_field`, `get_field_intensity`, `remove_field`). |
| Q2 Interface size | 3 | med | 107 pub-fn and 100 pub-data in 8 headers. Ordering rules: `on_field_modified` must dirty the transparency, seen and pathfinding caches ("now that field processing doesn't always do it", `map.cpp:6547`), `submap::field_count` has to be kept in sync, and newborn fields are skipped for one turn. |
| Q3 Locality | 3 | med | A new field behaviour stays inside `map_field.cpp` and `field_type.cpp`. But visibility (`lightmap.cpp`), danger for pathfinding (`map.cpp`) and per-ID special cases in Creatures files must change with it. |
| Q4 Seam cleanliness | 2 | med | Field state is stored inside `submap` (Terrain & Map), and processing is a set of methods of `map`. A seam needs field storage taken out of `submap` and the processors taken off `map`. There is no inheritance or `friend` leak. |
| Q5 Testability | 2 | med | One focused file, `field_test.cpp` (15 cases, 14 fixture calls). It drives `get_map().process_fields()` and advances the global `calendar::turn` by hand. `creature_in_field_test.cpp` uses the full map too. |

### Items

- **Files:** 73 files, 71,151 lines: `item.*`, `itype.*`, `item_factory`, `item_group`, `item_pocket`, `item_contents`, `item_location`, `visitable`, `iuse*`, `recipe*`, `crafting`, `requirements`, `ammo*`, `material`, `fault`, `relic` and `damage`.

| Criterion | Score | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 3 | med | `item`, `itype` (17 `islot_*` slots), `item_pocket`, `item_location` and `recipe` are homes. Item rules also live in Creatures: `character_inventory`, `character_attire`, `consumption.cpp` and `Character::wield`. They live in Terrain & Map as `map::process_items` and the active-item caches, and in Vehicles as `vehicle_stack`/`process_items_in_vehicles`. |
| M2 Language alignment | 3 | med | The upper-case legacy item types (`ARMOR`, `TOOL`, `GENERIC`, `COMESTIBLE`, `TOOL_ARMOR`, ...) map to one `itype` with optional slots, which is the right deep shape under drifting names. `item_group`, `pocket_data` and `material` match their JSON. "charges" means several things. |
| M3 Context isolation | 3 | med | glob 433 (7.7/kLOC, the lowest of the large contexts): `get_map()` 132, `calendar::turn` 82, `g->` 73. Many item functions take the carrier, map or position as parameters (`item::process( map &here, Character *carrier, ... )`). |
| Q1 Leverage | 2 | med | `item.h` has 603 public functions and 120 public data members, and `itype.h` has 336 public data members, mostly flat accessors. The deep parts are `item_contents` and `item_pocket` (pocket rules behind a few calls), `item_location` and `visitable`. |
| Q2 Interface size | 1 | high | 2,139 pub-fn and 1,042 pub-data. Hidden rules: active items must be registered in `active_item_cache`, containers must notify through `contents_change_handler`, and `item_location` is invalidated when items move. |
| Q3 Locality | 2 | med | Creatures declares 119 static `itype_id` IDs in 31 files, against 117 in Items' own 10 files. Item flags are referenced 952 times in Creatures and 1,331 in Items. Item serialization (`item`, `item_pocket`, `item_contents`) lives in `savegame_json.cpp`. |
| Q4 Seam cleanliness | 2 | med | Items' base classes are inherited outside the context: `visitable` by `Character`, `map_cursor`, `map_selector`, `vehicle_cursor` and `vehicle_selector`, and `item_stack` by `map_stack` and `vehicle_stack`. Items are stored in other contexts' containers (`cata::colony<item>` in `submap` and `vehicle_part`). `item` itself is a value type, which helps a seam. |
| Q5 Testability | 3 | med | 40 files with 265 cases, 19 of them fixture-free. Many tests just build `item( itype_id )` (`item_tname_test`, `comestible_test`, `requirements_test`), but crafting and reloading tests need the full avatar and map. |

### Vehicles

- **Files:** 22 files, 23,219 lines: `vehicle*.cpp`, `vehicle.h`, `veh_type.*`, `vpart_*`, `veh_utils`, `veh_appliance`, `turret`, `tileray`, `grab`.
- **Vehicle rules in `map.cpp`:** 34 vehicle functions; see Terrain & Map.

| Criterion | Score | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 2 | med | `vehicle`, `vehicle_part` and `vpart_info` have homes. But a core vehicle rule, movement and collision (`map::vehmove`, `vehproceed`, `move_vehicle`, `displace_vehicle`, `vehicle_vehicle_collision`), lives in `map.cpp`. Vehicle lookup (`veh_at`) is called 118 times from 23 Creatures files. |
| M2 Language alignment | 2 | high | The main concept is misnamed. JSON `vehicle_part` defines the part *type*, which C++ calls `vpart_info`, while C++ `vehicle_part` is the part *instance*. JSON `vehicle` is `vehicle_prototype`. So one word means two things. |
| M3 Context isolation | 2 | med | glob 246 (13.9/kLOC): `get_map()` 96, `get_player_character()` 67. Vehicle code checks the avatar's state directly. |
| Q1 Leverage | 2 | med | 685 pub-fn and 300 pub-data in 9 headers (`vehicle.h` has 493 public functions). The API is index-based (`part(int)`, `parts_at_relative`, `part_with_feature`), so callers do the real work. `vehicle::refresh()` and the power and fuel calls are the deep ones. |
| Q2 Interface size | 2 | med | Ordering rules: call `refresh()` after part changes, keep the map caches in sync by hand (`add_vehicle_to_cache`, `rebuild_vehicle_level_caches`; 38 calls in Terrain & Map, 8 in Vehicles), and dirty the transparency (8) and floor (5) caches from vehicle code. |
| Q3 Locality | 2 | med | Changing vehicle rules touches `map.cpp` (movement), `level_cache.h`, `savegame_json.cpp` (`vehicle` and `vehicle_part` serializers) and `veh_interact.cpp` in the Frontend. |
| Q4 Seam cleanliness | 1 | med | Vehicles are owned by `submap::vehicles` and moved by `map`. `level_cache` holds raw `vehicle*` sets. `vehicle.h` declares `friend` `veh_interact` (Frontend) and `item_location` (Items). The Items bases `item_stack` and `visitable` are inherited here (`vehicle_stack`, `vehicle_cursor`). |
| Q5 Testability | 3 | med | 12 files with 53 cases, and every file uses fixtures (71 calls; for example `vehicle_split_test` uses `get_map()` and `clear_vehicles()`). The tests go through public calls on a full map. |

### Time & Weather

- **Files:** 10 files, 4,134 lines: `calendar.*`, `weather.*`, `weather_gen.*`, `weather_type.*`, `timed_event.*`.

| Criterion | Score | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 4 | med | `time_point`, `time_duration`, seasons, sun and moon have one home in `calendar.h`/`calendar.cpp`, and other code uses them (`time_point`/`time_duration` appear 623 times in Creatures). The weather state has one owner, `weather_manager`. Its leaks are rules about other contexts that sit in `weather.h` (`wet_character`, `glare`, `retroactively_fill_from_funnel( item&, trap& )`, `warm_enough_to_plant`). |
| M2 Language alignment | 4 | med | `weather_type` (JSON) is `weather_type`, and JSON duration strings parse into `time_duration`. |
| M3 Context isolation | 3 | med | The highest density of any context (73 accesses, 23.9/kLOC), and `calendar::turn` is a mutable global (`extern time_point turn`, `calendar.h:140`). But it is written in only a few places: `do_turn.cpp` (`calendar::turn += 1_turns`) and tests. `weather_manager` is owned by `game` and reached through `get_weather()`. |
| Q1 Leverage | 4 | med | The time value types give a lot behind a small arithmetic interface (fan-in 167 files). The weather calls `update_weather()` and `get_temperature()` are deep. |
| Q2 Interface size | 3 | med | 151 pub-fn and 94 pub-data in 5 headers. Ordering rule: `weather.temperature_cache.clear()` every turn (`do_turn.cpp`). Presentation helpers leak in (`print_temperature`, `get_wind_color` returns `nc_color`). |
| Q3 Locality | 3 | med | A weather rule change also touches the temperature code in Character (`get_weather()` 34 times in Creatures), sight in `lightmap.cpp` (`sight_penalty`) and item rot. |
| Q4 Seam cleanliness | 3 | med | No inheritance or `friend` crosses it. The seam is blocked by two leaks: the `calendar::turn` global and `game`'s ownership of `weather_manager`. |
| Q5 Testability | 4 | med | 11 files with 52 cases and 0 fixture calls. `calendar_test`, `sun_test`, `moon_test`, `weather_test` and `temperature_test` call pure functions on a `time_point`. (The include-based test rule also assigned `units_test`, `event_test`, `health_test` and `rot_test` here, so the counts are a little inflated.) |

### Unclaimed

- **Files:** 237 files, 88,660 lines. A low Deep Model fit is expected here.
- **Contents by sub-group:**
  - Game loop and load/save: 9 files, 23,953 lines (`game`, `do_turn`, `init`, `savegame*`).
  - Scripting: 32 files, 11,807 lines (`effect_on_condition`, `condition`, `talker*`, `math_parser*`).
  - Events and telemetry: 19 files, 7,106 lines.
  - Utilities and infrastructure: 177 files, 45,794 lines (`json`, `flexbuffer`, `generic_factory`, `string_id`, `units`, `point`, `rng`, `translations`, ...).

| Criterion | Score | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 2 | med | `game` is a god object: 258 `game::` functions that span creatures, vehicles (`control_vehicle`, `remoteveh`), items, fields (`is_in_dangerous_field`), light (`natural_light_level`) and UI. Scripting does have a home: EOC, with 2,002 Content objects, in `effect_on_condition.*`. |
| M2 Language alignment | 3 | low | `effect_on_condition`, `talk_topic`/`talker` and `widget` match their JSON. The infrastructure names are generic. |
| M3 Context isolation | 1 | high | `game` is the global `g`: `g->` is used 177 times here and 337 in Creatures. `game` owns the map, the avatar, `creature_tracker`, `weather_manager`, `scent_map` and `timed_event_manager`, and `game.h` declares 24 `friend`s, among them `map`, `Character`, `avatar`, `creature_tracker`, `weather_manager` and `editmap`. |
| Q1 Leverage | 3 | med | `generic_factory` (1,356 lines behind `load`/`obj`/`is_valid`), `DynamicDataLoader`, `json`/`flexbuffer` and `string_id` are deep and are used everywhere (fan-in 487 files). `game.h` is shallow and wide (223 public functions, 68 public data members). |
| Q2 Interface size | 2 | med | 3,408 pub-fn and 426 pub-data, including utility templates. Load ordering in `init.cpp`: type registration, then finalize, then check, all in a hand-ordered list. |
| Q3 Locality | 2 | high | `savegame_json.cpp` (5,109 lines) holds 95 serializer definitions for classes from every context. `init.cpp` registers 164 loaders from every context. Adding a JSON type or a saved field means editing Unclaimed. |
| Q4 Seam cleanliness | 2 | med | The utilities are leaves that could sit behind a seam today. `game`, `do_turn` and `savegame*` are wired to every context, with globals in both directions and `friend`s. |
| Q5 Testability | 3 | med | 32 of 39 files are fixture-free and test utilities (`colony_test`, `flat_set_test`, `point_test`, `generic_factory_test`, `string_formatter_test`). The game loop and `do_turn` have no tests of their own. |

## 4. Summary matrix

Medians are taken over the criteria of each axis. The two axes are never merged.

| Context | M1 | M2 | M3 | Deep Model median | Q1 | Q2 | Q3 | Q4 | Q5 | Deep module median |
|---|---|---|---|---|---|---|---|---|---|---|
| Creatures | 3 | 3 | 1 | 3 | 2 | 1 | 2 | 1 | 3 | 2 |
| Terrain & Map | 3 | 3 | 2 | 3 | 3 | 1 | 2 | 1 | 3 | 2 |
| Fields | 3 | 4 | 2 | 3 | 4 | 3 | 3 | 2 | 2 | 3 |
| Items | 3 | 3 | 3 | 3 | 2 | 1 | 2 | 2 | 3 | 2 |
| Vehicles | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 1 | 3 | 2 |
| Time & Weather | 4 | 4 | 3 | 4 | 4 | 3 | 3 | 3 | 4 | 3 |
| Unclaimed | 2 | 3 | 1 | 2 | 3 | 2 | 2 | 2 | 3 | 2 |

No grand total.

## 5. FFI cut-cost notes

Recorded, not scored, against the four `cxx` weak spots.

- **Creatures:**
  - Containers: `std::map`/`std::unordered_map` throughout `character.h` (64 STL associative or `list` members, for example `my_mutations`, `vitamin_levels`, `moncams`, `inv_search_caches`), `std::list<shared_ptr_fast<npc>>` in `creature_tracker`, and the nested `std::unordered_map<int, std::unordered_map<mfaction_id, std::vector<shared_ptr_fast<Creature>>>>`.
  - Hierarchy: `Creature` with 176 virtuals across `monster` and `Character`→`avatar`/`npc`.
  - Shared state: raw `Creature*` pointers everywhere, and `shared_ptr_fast` ownership.
  - Hot loops: `monmove()` loops over every monster and calls `plan()`, `move()` and `map::creature_in_field()` (map and fields) on each, many times per turn.
- **Terrain & Map:**
  - Containers: `cata::mdarray` caches (fixed arrays, FFI-friendly), but `submap` holds `cata::colony<item>`, `std::map<point, computer>`, `std::map<tripoint_sm_ms, partial_con>` and `std::vector<std::unique_ptr<vehicle>>`.
  - Shared state: `level_cache` has `std::set<vehicle*>` and `std::unordered_map<tripoint, std::pair<vehicle*, int>>`.
  - Hot loops: `map::route` A*, cache builds (`build_transparency_cache`, `build_seen_cache`, `generate_lightmap`) and `map::sees`, all called per creature per turn.
- **Fields:**
  - Containers: `field` is a `lazy<std::map<field_type_id, field_entry>>` per tile.
  - Hierarchy: none.
  - Hot loops: `process_fields_in_submap` loops over every tile of every submap and calls processor function pointers that read the map, creatures and scent. That loop would sit on the far side of a seam unless storage moves with it.
- **Items:**
  - Containers: `std::list<item>` in `item_pocket::contents`, `cata::colony<item>` in map and vehicle storage, and `item_location` with pointers into map, vehicle and character storage.
  - Hierarchy: the `visitable`/`item_stack` bases are implemented by Creatures, Map and Vehicles.
  - Hot loops: `map::process_items` → `item::process` per active item.
- **Vehicles:**
  - Containers: `std::vector<vehicle_part>`, `std::map<point, std::vector<int>> relative_parts`, `std::map<itype_id, units::energy>`, and `cata::colony<item>` per part.
  - Shared state: raw `vehicle*` in the map caches.
  - Hot loops: `map::vehmove` and the collision checks per moving vehicle.
- **Time & Weather:** `time_point`/`time_duration` are trivially copyable `int` wrappers (cheap to cross). Its cost is the `calendar::turn` global, read on both sides.
- **Unclaimed:**
  - Globals: `g` and the `get_*()` singletons are the main FFI hazard; they would need a context handle passed across.
  - Containers: `generic_factory` is template-heavy (`std::unordered_map<string_id<T>, int_id<T>>` plus vectors), so it is not FFI-friendly. On the Rust side it would be reimplemented, not wrapped.

## 6. Cross-cutting seams

CDDA 0.H runs Simulation on a single thread. The only `std::thread` use is in `sounds.cpp` (SFX). These are the 2023–24 performance structures present in the tree, identified from code and comments:

1. **Vision transparency cache and the visibility cache chain.**
   - **What it is:** `level_cache` (`level_cache.h`) adds `vision_transparency_cache`, an "adjusted transparency" copy of `transparency_cache` that accounts for the player's posture (crouching, prone, all fours). It sits beside `transparent_cache_wo_fields`, the bitset `field_cache`, `seen_cache`/`camera_cache`, the two `reachability_cache`s held through `value_ptr` (the comment explains this keeps `level_cache` CPU-cache friendly), and the map-memory caches. Builders: `map::build_transparency_cache` (`lightmap.cpp:87`) has a fast path for uniform submaps and per-submap dirty bits, `map::build_vision_transparency_cache` is at `lightmap.cpp:192`, and `map::build_map_cache` (`map.cpp:9384`) orchestrates.
   - **Contexts crossed:**
     - Terrain & Map: terrain and furniture transparency.
     - Fields: `field_intensity_level::translucency`.
     - Time & Weather: `get_weather().weather_id->sight_penalty`.
     - Creatures: `get_player_character()` posture, `avatar::get_active_moncams()`.
     - Vehicles: `do_vehicle_caching`, plus vehicle code that dirties the transparency, seen and floor caches 17 times.
     - Frontend: `tilecontext->set_draw_cache_dirty()` under `#if defined(TILES)` inside `build_map_cache`.
   - **Invalidation:** fully manual. `set_transparency_cache_dirty` is called from 4 Terrain & Map files, 3 Vehicles files, 1 Time & Weather file and 1 Frontend file. Fields dirty it through `map::on_field_modified`.
   - **Implication:** a natural candidate for a **Visibility** module with a narrow input ("opacity changed at p") and a narrow output (`sees`, `seen_cache`). Today its seam would cross six contexts.
2. **Monster planning.**
   - **Planning code:** `monster::plan()` (`monmove.cpp:452`) collects its working state in a `monster_plan` struct and splits the anger and fear rules into helpers (`anger_hostile_seen`, `anger_cub_threatened`, ...).
   - **Target search:** it queries `creature_tracker::for_each_reachable( origin, faction_pred, visit )`. This is backed by `creatures_by_zone_and_faction_`, which `flood_fill_zone` builds over the map's reachability, and is rebuilt only when `dirty_` is set (`creature_tracker.h`).
   - **Throttling:** monsters with no target rate-limit hostile scans with `turns_since_target`, skipping up to 600 turns on a logarithmic ramp (`max_turns_for_rate_limiting = 1800`). It also calls `map::get_inter_level_visibility()` to prune z-levels.
   - **Contexts crossed:** Creatures (planning and tracker), Terrain & Map (reachability, `sees`, and `invalidate_reachability_cache` called 4 times from `map.cpp`), and Unclaimed (`game` owns the tracker and is its `friend`; `monmove()` in `do_turn.cpp` drives the loop).
   - **Implication:** the throttling is Baseline behaviour, not something the port adds, but it is exactly the kind of rule an **Approximation** discussion has to know about. `do_turn.cpp` also says, before `build_map_cache( levz, true )`: "If this turns out to be expensive, consider a stripped down cache just for monsters."
3. **Pathfinder.**
   - **What it is:** still the classic A* `map::route` in `pathfinding.cpp` (582 lines). It uses flat `path_data_layer` bitsets and arrays per z-level and a per-z `pathfinding_cache` of `pf_special` bit flags with dirty points (`pathfinding.h`). No newer pathfinder is present in the 0.H tree.
   - **Contexts crossed:**
     - The cache is dirtied by Terrain & Map (`set_pathfinding_cache_dirty`, 12 calls in `map.cpp`), by Fields (dangerous fields, through `on_field_modified`), and by traps and vehicles (`PF_TRAP`, `PF_VEHICLE`).
     - Callers are in Creatures: `.route(` 17 times in 8 files (`monmove`, `npcmove`, activities).
     - `pathfinding.h` includes `character.h` (for `creature_size`), so the map's pathfinder depends on a Creatures header.
   - **Overmap routing:** `simple_pathfinding` is separate and clean (`simple_pathfinding_test`, 4 fixture-free cases). `map::route` has no direct tests.
4. **The turn sequence as an implicit seam.**
   - **Per-turn order** (`do_turn.cpp`): timed events → missions → hordes (`overmap_buffer`) → weather → player input loop (`g->handle_action()`, so Frontend is interleaved with Simulation) → scent → `build_floor_caches` → falling → `vehmove` → `process_fields` → `process_items` → explosions → sounds → `build_map_cache( z, skip_lightmap )` → `monmove` → emissions every 10 s.
   - **Why it matters:** every context depends on this hand-ordered list. A port that moves one context across a seam must keep the order: for example, fields before items, and caches rebuilt before monsters plan.

## 7. Context proposals

Evidence for each is in sections 2, 3 and 6.

- **Split Creatures** (162k lines, a third of the Engine) into:
  - **Character** (109 files, 91k lines): Character, avatar, body, mutations, bionics, proficiencies, activities.
  - **Monsters** (36 files, 22.5k lines): `monster`, `mtype`, `mattack*`, `mongroup`, `monmove`, behaviour.
  - **NPC society** (39 files, 37k lines): `npc*`, dialogue, missions, factions, basecamp.
  - **Combat** (14 files, 11k lines): `melee`, `ranged`, `ballistics`, `projectile`, `dispersion`, martial arts.

  Monsters and Character share only `Creature`, and that base class (176 virtuals) is the seam to design. The monster planning hot path lives entirely inside the Monsters sub-context plus `creature_tracker`.
- **New context: Visibility & Lighting**, carved out of Terrain & Map: `lightmap`, `shadowcasting`, `level_cache`'s vision members, `reachability_cache`, and `map::sees`. Only 10 files and 3.8k lines, but it is the most cross-cutting structure (section 6.1) and a likely Dense-area hot spot.
- **Split Terrain & Map** into the **Reality bubble** (map, submap, mapbuffer, terrain and furniture; 35k lines) and **Worldgen & Overmap** (mapgen, overmap, city, regional settings, map extras; 34 files, 29k lines). Worldgen is not per-turn Simulation (except hordes in `overmap_buffer`) and talks to the bubble through `mapgendata`/`tinymap`.
- **New context: Scripting**, taken from Unclaimed: EOC, conditions, `talker*` (an adapter per creature, item and furniture) and `math_parser*`. 32 files, 11.8k lines, 2,002 EOC Content objects. Its `talker` hierarchy is already a real seam with several adapters (`talker_avatar`, `talker_npc`, `talker_monster`, `talker_item`, `talker_furniture`).
- **Move vehicle movement into Vehicles:** the 34 vehicle `map::` functions in `map.cpp` belong there, so the context can have a seam at all.
- **Fields: keep it as a model context, but plan storage with Terrain & Map.** At 4.5k lines it is tiny, but it has the best Deep module shape among the Simulation contexts (one deep entry point, data-driven processors). Its seam needs field storage taken out of `submap`.
- **Time & Weather: split Time from Weather.** Time is a near-ideal leaf value module (fan-in 167). Weather is a small owner with leaks into Creatures and Items. Splitting them lets Time go across first.

## 8. Open doubts

- **The literal split rule.** It puts `game.cpp`, `character.cpp`, `map.cpp` and others in Frontend because they include `cursesdef.h`. This report scores on an adjusted rule; the calibration pass must check that the Bright Nights report handles this the same way.
- **Doubtful assignments.** Moving the doubtful files (section 2) would shift the per-context line counts by about 10% but not the medians. The biggest are NPC society in Creatures and `iexamine.cpp` in Terrain & Map.
- **Interface counts.** These are regex estimates. Templates, macros and multi-line declarations are miscounted.
- **Reads versus writes.** The static view can't separate reads from writes for `get_map()`/`g->`; the writes row only catches direct assignment.
- **What counts as 2023–24 work.** Without history, the vision transparency cache, the `creature_tracker` reachability zones and the `monster_plan` throttling are dated by code shape and comments only. Some may be older.
- **Pathfinder.** No newer pathfinder is in this tree, so if the ticket assumed a 2023–24 pathfinder rewrite inside 0.H, that rewrite did not ship in 0.H. This can't be confirmed without history.
- **Test assignment.** Tests were assigned by name first and include majority second. Several assignments are wrong (for example `units_test` went to Time & Weather), so the test counts are indicative only.
- **Runtime cost.** Which hot loop dominates in Dense areas isn't visible statically; profiling decides that.
