//! Structured metadata for every name a Lua cart script can call — the
//! console's own builtins (registered in [`super::lua_exec::register_builtins`])
//! plus the Lua stdlib members this console leans on. Single source of truth
//! for editor tooling (autocomplete, hover docs, signature help); the
//! syntax-highlighter's builtin list in `caiven-studio`'s code panel is
//! derived from [`all_names`] so the two can't drift apart.

pub struct Param {
    pub name: &'static str,
    pub ty: &'static str,
}

pub struct ApiEntry {
    pub name: &'static str,
    pub params: &'static [Param],
    pub returns: &'static str,
    pub doc: &'static str,
}

macro_rules! param {
    ($name:literal : $ty:literal) => {
        Param {
            name: $name,
            ty: $ty,
        }
    };
}

/// Console builtins — mirrors `register_builtins` in `lua_exec.rs` exactly;
/// keep in sync when that function's signatures change.
pub const BUILTINS: &[ApiEntry] = &[
    ApiEntry {
        name: "clear_screen",
        params: &[],
        returns: "nil",
        doc: "Clear the world and UI layers to transparent.",
    },
    ApiEntry {
        name: "set_pixel",
        params: &[param!("x": "number"), param!("y": "number"), param!("color_index": "u8")],
        returns: "nil",
        doc: "Set a single pixel to a palette color.",
    },
    ApiEntry {
        name: "sprite",
        params: &[param!("sprite_id": "u8"), param!("x": "number"), param!("y": "number")],
        returns: "nil",
        doc: "Draw sprite sprite_id with its top-left at (x, y), camera-relative.",
    },
    ApiEntry {
        name: "button_down",
        params: &[param!("button_index": "u8")],
        returns: "bool",
        doc: "True while button_index is held down.",
    },
    ApiEntry {
        name: "button_pressed",
        params: &[param!("button_index": "u8")],
        returns: "bool",
        doc: "True on the single frame button_index was first pressed.",
    },
    ApiEntry {
        name: "draw_text",
        params: &[
            param!("text": "string"),
            param!("x": "number"),
            param!("y": "number"),
            param!("color_index": "u8"),
        ],
        returns: "nil",
        doc: "Draw text on the UI layer at (x, y).",
    },
    ApiEntry {
        name: "draw_number",
        params: &[
            param!("value": "number"),
            param!("x": "number"),
            param!("y": "number"),
            param!("color_index": "u8"),
        ],
        returns: "nil",
        doc: "Draw an integer on the UI layer at (x, y).",
    },
    ApiEntry {
        name: "fill_screen",
        params: &[param!("color_index": "u8")],
        returns: "nil",
        doc: "Fill the entire world layer with one color.",
    },
    ApiEntry {
        name: "draw_line",
        params: &[
            param!("x0": "number"),
            param!("y0": "number"),
            param!("x1": "number"),
            param!("y1": "number"),
            param!("color_index": "u8"),
        ],
        returns: "nil",
        doc: "Draw a line from (x0, y0) to (x1, y1), camera-relative.",
    },
    ApiEntry {
        name: "draw_rect",
        params: &[
            param!("x": "number"),
            param!("y": "number"),
            param!("w": "number"),
            param!("h": "number"),
            param!("color_index": "u8"),
        ],
        returns: "nil",
        doc: "Draw a rectangle outline, camera-relative.",
    },
    ApiEntry {
        name: "fill_rect",
        params: &[
            param!("x": "number"),
            param!("y": "number"),
            param!("w": "number"),
            param!("h": "number"),
            param!("color_index": "u8"),
        ],
        returns: "nil",
        doc: "Draw a filled rectangle, camera-relative.",
    },
    ApiEntry {
        name: "draw_circle",
        params: &[
            param!("cx": "number"),
            param!("cy": "number"),
            param!("r": "number"),
            param!("color_index": "u8"),
        ],
        returns: "nil",
        doc: "Draw a circle outline, camera-relative.",
    },
    ApiEntry {
        name: "fill_circle",
        params: &[
            param!("cx": "number"),
            param!("cy": "number"),
            param!("r": "number"),
            param!("color_index": "u8"),
        ],
        returns: "nil",
        doc: "Draw a filled circle, camera-relative.",
    },
    ApiEntry {
        name: "set_camera",
        params: &[param!("x": "number"), param!("y": "number")],
        returns: "nil",
        doc: "Set the camera's world-space offset.",
    },
    ApiEntry {
        name: "set_palette_color",
        params: &[
            param!("index": "number"),
            param!("r": "u8"),
            param!("g": "u8"),
            param!("b": "u8"),
        ],
        returns: "nil",
        doc: "Set palette slot index to an RGB color.",
    },
    ApiEntry {
        name: "draw_map",
        params: &[
            param!("cx": "number"),
            param!("cy": "number"),
            param!("sx": "number"),
            param!("sy": "number"),
            param!("w": "number"),
            param!("h": "number"),
        ],
        returns: "nil",
        doc: "Draw a w x h block of map tiles starting at cell (cx, cy) to screen position (sx, sy).",
    },
    ApiEntry {
        name: "get_tile",
        params: &[param!("x": "number"), param!("y": "number")],
        returns: "u8",
        doc: "Read the tile id at map cell (x, y); 0 if out of bounds.",
    },
    ApiEntry {
        name: "set_tile",
        params: &[param!("x": "number"), param!("y": "number"), param!("tile": "u8")],
        returns: "nil",
        doc: "Write a tile id at map cell (x, y); no-op if out of bounds.",
    },
    ApiEntry {
        name: "get_sprite_flags",
        params: &[param!("sprite_id": "u8")],
        returns: "u8",
        doc: "Read the per-sprite flag byte for sprite_id.",
    },
    ApiEntry {
        name: "set_sprite_flags",
        params: &[param!("sprite_id": "u8"), param!("flags": "u8")],
        returns: "nil",
        doc: "Write the per-sprite flag byte for sprite_id.",
    },
    ApiEntry {
        name: "play_sfx",
        params: &[param!("id": "u8")],
        returns: "nil",
        doc: "Start sound effect id.",
    },
    ApiEntry {
        name: "play_music",
        params: &[param!("id": "u8")],
        returns: "nil",
        doc: "Start music track id, looping.",
    },
    ApiEntry {
        name: "stop_music",
        params: &[],
        returns: "nil",
        doc: "Stop the currently playing music track.",
    },
    ApiEntry {
        name: "real_time",
        params: &[],
        returns: "(u8, u8, u8)",
        doc: "Read the real-time clock as (hour, minute, second).",
    },
];

/// Lua stdlib members this console leans on — never Rust-registered (see
/// `lua_exec.rs`'s module doc comment), so hand-authored here rather than
/// derived from anything.
pub const STDLIB: &[ApiEntry] = &[
    ApiEntry {
        name: "math.abs",
        params: &[param!("x": "number")],
        returns: "number",
        doc: "Absolute value of x.",
    },
    ApiEntry {
        name: "math.floor",
        params: &[param!("x": "number")],
        returns: "number",
        doc: "Largest integer <= x.",
    },
    ApiEntry {
        name: "math.ceil",
        params: &[param!("x": "number")],
        returns: "number",
        doc: "Smallest integer >= x.",
    },
    ApiEntry {
        name: "math.sqrt",
        params: &[param!("x": "number")],
        returns: "number",
        doc: "Square root of x.",
    },
    ApiEntry {
        name: "math.sin",
        params: &[param!("x": "number")],
        returns: "number",
        doc: "Sine of x (radians).",
    },
    ApiEntry {
        name: "math.cos",
        params: &[param!("x": "number")],
        returns: "number",
        doc: "Cosine of x (radians).",
    },
    ApiEntry {
        name: "math.max",
        params: &[param!("...": "number")],
        returns: "number",
        doc: "Largest of the given numbers.",
    },
    ApiEntry {
        name: "math.min",
        params: &[param!("...": "number")],
        returns: "number",
        doc: "Smallest of the given numbers.",
    },
    ApiEntry {
        name: "math.random",
        params: &[param!("m?": "number"), param!("n?": "number")],
        returns: "number",
        doc: "Random number: [0,1) with no args, [1,m] with one, [m,n] with two.",
    },
    ApiEntry {
        name: "math.huge",
        params: &[],
        returns: "number",
        doc: "Floating-point infinity.",
    },
    ApiEntry {
        name: "string.sub",
        params: &[param!("s": "string"), param!("i": "number"), param!("j?": "number")],
        returns: "string",
        doc: "Substring from index i to j (inclusive, 1-based).",
    },
    ApiEntry {
        name: "string.len",
        params: &[param!("s": "string")],
        returns: "number",
        doc: "Length of s in bytes.",
    },
    ApiEntry {
        name: "string.format",
        params: &[param!("fmt": "string"), param!("...": "any")],
        returns: "string",
        doc: "printf-style string formatting.",
    },
    ApiEntry {
        name: "string.find",
        params: &[param!("s": "string"), param!("pattern": "string"), param!("init?": "number")],
        returns: "number, number",
        doc: "Start/end indices of the first pattern match, or nil.",
    },
    ApiEntry {
        name: "string.gsub",
        params: &[
            param!("s": "string"),
            param!("pattern": "string"),
            param!("repl": "string"),
            param!("n?": "number"),
        ],
        returns: "string, number",
        doc: "Replace occurrences of pattern with repl; returns result and count.",
    },
    ApiEntry {
        name: "string.match",
        params: &[param!("s": "string"), param!("pattern": "string"), param!("init?": "number")],
        returns: "string",
        doc: "First match of pattern in s, or nil.",
    },
    ApiEntry {
        name: "string.rep",
        params: &[param!("s": "string"), param!("n": "number")],
        returns: "string",
        doc: "s repeated n times.",
    },
    ApiEntry {
        name: "string.upper",
        params: &[param!("s": "string")],
        returns: "string",
        doc: "s converted to upper case.",
    },
    ApiEntry {
        name: "string.lower",
        params: &[param!("s": "string")],
        returns: "string",
        doc: "s converted to lower case.",
    },
    ApiEntry {
        name: "table.insert",
        params: &[param!("t": "table"), param!("pos?": "number"), param!("value": "any")],
        returns: "nil",
        doc: "Insert value into t, at pos if given, else at the end.",
    },
    ApiEntry {
        name: "table.remove",
        params: &[param!("t": "table"), param!("pos?": "number")],
        returns: "any",
        doc: "Remove and return the element at pos (default: last).",
    },
    ApiEntry {
        name: "table.concat",
        params: &[
            param!("t": "table"),
            param!("sep?": "string"),
            param!("i?": "number"),
            param!("j?": "number"),
        ],
        returns: "string",
        doc: "Concatenate t[i..j] with sep between elements.",
    },
    ApiEntry {
        name: "table.sort",
        params: &[param!("t": "table"), param!("comp?": "function")],
        returns: "nil",
        doc: "Sort t in place, optionally with a custom comparator.",
    },
];

pub fn lookup(name: &str) -> Option<&'static ApiEntry> {
    BUILTINS
        .iter()
        .chain(STDLIB.iter())
        .find(|e| e.name == name)
}

pub fn all_names() -> impl Iterator<Item = &'static str> {
    BUILTINS.iter().chain(STDLIB.iter()).map(|e| e.name)
}
