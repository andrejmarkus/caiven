//! Structured metadata for every name a Lua cart script can call — the
//! console's own builtins (registered in [`super::lua_exec::register_builtins`]),
//! the pure-Lua gameplay stdlib (`lua_exec.rs`'s `prelude/*.lua`), plus the Lua
//! stdlib members this console leans on. Single source of truth for editor
//! tooling (autocomplete, hover docs, signature help); the
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
        params: &[
            param!("x": "number"),
            param!("y": "number"),
            param!("color_index": "u8"),
        ],
        returns: "nil",
        doc: "Set a single pixel to a palette color.",
    },
    ApiEntry {
        name: "sprite",
        params: &[
            param!("sprite_id": "u8"),
            param!("x": "number"),
            param!("y": "number"),
            param!("flip_x": "bool?"),
            param!("flip_y": "bool?"),
            param!("rotate": "number?"),
        ],
        returns: "nil",
        doc: "Draw sprite sprite_id with its top-left at (x, y), camera-relative. flip_x/flip_y mirror the sprite (default false); rotate is 0/90/180/270 degrees clockwise (default 0, any other value is a Lua error). Rotation is applied before flipping.",
    },
    ApiEntry {
        name: "button_down",
        params: &[param!("button_index": "u8")],
        returns: "bool",
        doc: "True while button_index is held down. 0=Up 1=Down 2=Left 3=Right 4=A 5=B 6=Select. START is reserved by the console for its pause menu and never reaches a cart.",
    },
    ApiEntry {
        name: "button_pressed",
        params: &[param!("button_index": "u8")],
        returns: "bool",
        doc: "True on the single frame button_index was first pressed. Same indices as button_down; an out-of-range index is always false.",
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
        params: &[
            param!("x": "number"),
            param!("y": "number"),
            param!("tile": "u8"),
        ],
        returns: "nil",
        doc: "Write a tile id at map cell (x, y); no-op if out of bounds.",
    },
    ApiEntry {
        name: "get_collision",
        params: &[param!("tx": "number"), param!("ty": "number")],
        returns: "u8",
        doc: "Read the collision-type id at map cell (tx, ty); 0 if out of bounds. The id indexes the cart's collision-type table (0/1/2 built-in as walkable/solid/hazard, 3-255 free for custom types) — see collision_type_name/collision_is_solid.",
    },
    ApiEntry {
        name: "set_collision",
        params: &[
            param!("tx": "number"),
            param!("ty": "number"),
            param!("value": "u8"),
        ],
        returns: "nil",
        doc: "Write the collision-type id at map cell (tx, ty); no-op if out of bounds.",
    },
    ApiEntry {
        name: "collision_type_id",
        params: &[param!("name": "string")],
        returns: "u8",
        doc: "Look up a collision type's id by name; 0 (walkable) if no type has that name.",
    },
    ApiEntry {
        name: "collision_type_name",
        params: &[param!("id": "u8")],
        returns: "string",
        doc: "Look up a collision type's name by id; \"\" if the id is undefined.",
    },
    ApiEntry {
        name: "collision_is_solid",
        params: &[param!("id": "u8")],
        returns: "bool",
        doc: "True if the collision type with this id is flagged solid. Used by tile_solid/box_touches_solid; undefined ids are never solid.",
    },
    ApiEntry {
        name: "load_sprite_bank",
        params: &[param!("id": "u8")],
        returns: "bool",
        doc: "Switch the sprite RAM window to bank id; false when it does not exist.",
    },
    ApiEntry {
        name: "load_map_bank",
        params: &[param!("id": "u8")],
        returns: "bool",
        doc: "Switch the map RAM window to bank id; false when it does not exist.",
    },
    ApiEntry {
        name: "load_palette_bank",
        params: &[param!("id": "u8")],
        returns: "bool",
        doc: "Switch the palette RAM window to bank id; false when it does not exist.",
    },
    ApiEntry {
        name: "load_sfx_bank",
        params: &[param!("id": "u8")],
        returns: "bool",
        doc: "Switch the SFX RAM window to bank id; false when it does not exist.",
    },
    ApiEntry {
        name: "load_music_bank",
        params: &[param!("id": "u8")],
        returns: "bool",
        doc: "Switch the music RAM window to bank id; false when it does not exist.",
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
        name: "dset",
        params: &[param!("slot": "u8"), param!("value": "number")],
        returns: "nil",
        doc: "Write value into save slot 0-63. Errors if slot is out of range. Persists across sessions once the host flushes it to disk.",
    },
    ApiEntry {
        name: "dget",
        params: &[param!("slot": "u8")],
        returns: "number",
        doc: "Read save slot 0-63; 0 if never set. Errors if slot is out of range.",
    },
    ApiEntry {
        name: "save_data",
        params: &[param!("data": "table")],
        returns: "nil",
        doc: "Replace the persisted save blob with data (string/number/bool/nested-table keys and values only). Errors if the packed size exceeds 4KiB or a value can't be serialized.",
    },
    ApiEntry {
        name: "load_data",
        params: &[],
        returns: "table",
        doc: "Return the persisted save blob, or {} if save_data has never been called.",
    },
    ApiEntry {
        name: "real_time",
        params: &[],
        returns: "(u8, u8, u8)",
        doc: "Read the real-time clock as (hour, minute, second).",
    },
    ApiEntry {
        name: "frame_count",
        params: &[],
        returns: "number",
        doc: "Number of frames run since the cart loaded.",
    },
    ApiEntry {
        name: "time",
        params: &[],
        returns: "number",
        doc: "Seconds since the cart loaded, assuming 60 frames per second.",
    },
];

/// Gameplay-facing stdlib — pure Lua (`lua_exec.rs`'s `prelude/*.lua`), not
/// Rust-registered, so hand-authored here like `STDLIB` below rather than
/// derived from anything.
pub const PRELUDE: &[ApiEntry] = &[
    ApiEntry {
        name: "lerp",
        params: &[
            param!("a": "number"),
            param!("b": "number"),
            param!("t": "number"),
        ],
        returns: "number",
        doc: "Linear interpolation from a to b at t (0..1).",
    },
    ApiEntry {
        name: "clamp",
        params: &[
            param!("v": "number"),
            param!("lo": "number"),
            param!("hi": "number"),
        ],
        returns: "number",
        doc: "v restricted to the [lo, hi] range.",
    },
    ApiEntry {
        name: "ease_linear",
        params: &[param!("t": "number")],
        returns: "number",
        doc: "Identity easing curve: ease_linear(t) == t.",
    },
    ApiEntry {
        name: "ease_in_quad",
        params: &[param!("t": "number")],
        returns: "number",
        doc: "Quadratic ease-in curve over t (0..1).",
    },
    ApiEntry {
        name: "ease_out_quad",
        params: &[param!("t": "number")],
        returns: "number",
        doc: "Quadratic ease-out curve over t (0..1).",
    },
    ApiEntry {
        name: "ease_in_out_quad",
        params: &[param!("t": "number")],
        returns: "number",
        doc: "Quadratic ease-in-then-out curve over t (0..1).",
    },
    ApiEntry {
        name: "aabb_overlap",
        params: &[
            param!("x1": "number"),
            param!("y1": "number"),
            param!("w1": "number"),
            param!("h1": "number"),
            param!("x2": "number"),
            param!("y2": "number"),
            param!("w2": "number"),
            param!("h2": "number"),
        ],
        returns: "bool",
        doc: "True if the two axis-aligned boxes overlap.",
    },
    ApiEntry {
        name: "tile_solid",
        params: &[param!("tx": "number"), param!("ty": "number")],
        returns: "bool",
        doc: "True if the collision type at (tx, ty) is flagged solid (see collision_is_solid) — any type with the SOLID flag, not just the built-in id 1.",
    },
    ApiEntry {
        name: "box_touches_solid",
        params: &[
            param!("x": "number"),
            param!("y": "number"),
            param!("w": "number"),
            param!("h": "number"),
        ],
        returns: "bool",
        doc: "True if the pixel-space box overlaps any solid map tile.",
    },
    ApiEntry {
        name: "new_tween",
        params: &[
            param!("from": "number"),
            param!("to": "number"),
            param!("frames": "number"),
            param!("ease?": "function"),
        ],
        returns: "table",
        doc: "Creates tween state; ease defaults to ease_linear.",
    },
    ApiEntry {
        name: "tween_update",
        params: &[param!("tw": "table")],
        returns: "number",
        doc: "Advances tw by one frame and returns its current value; tw.done flips true on arrival.",
    },
    ApiEntry {
        name: "new_anim",
        params: &[param!("frames": "table"), param!("frame_len": "number")],
        returns: "table",
        doc: "Creates animation state cycling through a list of sprite ids.",
    },
    ApiEntry {
        name: "anim_update",
        params: &[param!("anim": "table")],
        returns: "nil",
        doc: "Advances anim by one frame, looping back to the first frame at the end.",
    },
    ApiEntry {
        name: "anim_sprite",
        params: &[param!("anim": "table")],
        returns: "number",
        doc: "The sprite id anim is currently showing.",
    },
    ApiEntry {
        name: "Particles.spawn",
        params: &[
            param!("x": "number"),
            param!("y": "number"),
            param!("vx": "number"),
            param!("vy": "number"),
            param!("color": "u8"),
            param!("life": "number"),
        ],
        returns: "nil",
        doc: "Spawns a particle with the given position, velocity, palette color, and lifetime in frames.",
    },
    ApiEntry {
        name: "Particles.update",
        params: &[],
        returns: "nil",
        doc: "Advances all particles by one frame, dropping any past their lifetime.",
    },
    ApiEntry {
        name: "Particles.draw",
        params: &[],
        returns: "nil",
        doc: "Draws every live particle as a single pixel.",
    },
    ApiEntry {
        name: "Particles.clear",
        params: &[],
        returns: "nil",
        doc: "Removes all particles.",
    },
    ApiEntry {
        name: "Particles.count",
        params: &[],
        returns: "number",
        doc: "Number of live particles.",
    },
    ApiEntry {
        name: "Vec2.new",
        params: &[param!("x": "number"), param!("y": "number")],
        returns: "Vec2",
        doc: "Construct a 2D vector. Supports +, -, unary -, * (Vec2 * number or number * Vec2), and == (component equality). tostring(v) gives \"(x, y)\".",
    },
    ApiEntry {
        name: "Vec2:length",
        params: &[],
        returns: "number",
        doc: "Magnitude of the vector.",
    },
    ApiEntry {
        name: "Vec2:length_squared",
        params: &[],
        returns: "number",
        doc: "Squared magnitude — avoids a sqrt when only comparing magnitudes.",
    },
    ApiEntry {
        name: "Vec2:normalize",
        params: &[],
        returns: "Vec2",
        doc: "Unit-length copy of the vector. A zero-length vector returns Vec2.new(0, 0), not an error.",
    },
    ApiEntry {
        name: "Vec2:dot",
        params: &[param!("other": "Vec2")],
        returns: "number",
        doc: "Dot product with another Vec2.",
    },
    ApiEntry {
        name: "Vec2:distance",
        params: &[param!("other": "Vec2")],
        returns: "number",
        doc: "Distance to another Vec2.",
    },
    ApiEntry {
        name: "random_range",
        params: &[param!("lo": "number"), param!("hi": "number")],
        returns: "number",
        doc: "Random integer in [lo, hi], inclusive. Deterministic per cart run unless the cart calls math.randomseed().",
    },
    ApiEntry {
        name: "random_float",
        params: &[param!("lo": "number"), param!("hi": "number")],
        returns: "number",
        doc: "Random float in [lo, hi).",
    },
    ApiEntry {
        name: "choice",
        params: &[param!("t": "table")],
        returns: "any",
        doc: "Random element of a non-empty array-like table. Errors on an empty table.",
    },
    ApiEntry {
        name: "shuffle",
        params: &[param!("t": "table")],
        returns: "table",
        doc: "Fisher-Yates shuffle of t, in place. Returns t.",
    },
    ApiEntry {
        name: "circle_overlap",
        params: &[
            param!("x1": "number"),
            param!("y1": "number"),
            param!("r1": "number"),
            param!("x2": "number"),
            param!("y2": "number"),
            param!("r2": "number"),
        ],
        returns: "bool",
        doc: "Whether two circles overlap. Exactly-tangent circles (distance == sum of radii) count as not overlapping.",
    },
    ApiEntry {
        name: "point_in_rect",
        params: &[
            param!("px": "number"),
            param!("py": "number"),
            param!("x": "number"),
            param!("y": "number"),
            param!("w": "number"),
            param!("h": "number"),
        ],
        returns: "bool",
        doc: "Whether (px, py) is inside the rect (x, y, w, h). The left/top edges count as inside; the right/bottom edges don't (half-open, matching aabb_overlap's convention).",
    },
    ApiEntry {
        name: "point_in_circle",
        params: &[
            param!("px": "number"),
            param!("py": "number"),
            param!("cx": "number"),
            param!("cy": "number"),
            param!("r": "number"),
        ],
        returns: "bool",
        doc: "Whether (px, py) is inside or exactly on the circle centered at (cx, cy) with radius r.",
    },
    ApiEntry {
        name: "Sprite.new",
        params: &[param!("opts": "table")],
        returns: "Sprite",
        doc: "Bundle a sprite_id, Vec2 pos, and optional flip_x/flip_y/rotate (defaults false/false/0) into one drawable object. opts = { sprite_id, pos, flip_x, flip_y, rotate }.",
    },
    ApiEntry {
        name: "Sprite:draw",
        params: &[],
        returns: "nil",
        doc: "Draw the sprite at its current pos via the sprite() builtin. Move it by reassigning .pos (e.g. s.pos = s.pos + v).",
    },
    ApiEntry {
        name: "Scenes.push",
        params: &[param!("scene": "table")],
        returns: "nil",
        doc: "Calls scene.enter(scene) if present, then pushes scene onto the top of the stack.",
    },
    ApiEntry {
        name: "Scenes.pop",
        params: &[],
        returns: "nil",
        doc: "Calls the top scene's exit(scene) if present, then removes it. Errors if the stack is empty.",
    },
    ApiEntry {
        name: "Scenes.switch",
        params: &[param!("scene": "table")],
        returns: "nil",
        doc: "Pops the current top scene (calling its exit) and pushes scene (calling its enter) in its place. Errors if the stack is empty.",
    },
    ApiEntry {
        name: "Scenes.update",
        params: &[],
        returns: "nil",
        doc: "Calls the top scene's update(scene) if present. A no-op on an empty stack.",
    },
    ApiEntry {
        name: "Scenes.draw",
        params: &[],
        returns: "nil",
        doc: "Calls the top scene's draw(scene) if present. A no-op on an empty stack.",
    },
    ApiEntry {
        name: "Scenes.current",
        params: &[],
        returns: "table?",
        doc: "The scene table on top of the stack, or nil if the stack is empty.",
    },
    ApiEntry {
        name: "Entities.add",
        params: &[param!("e": "table")],
        returns: "nil",
        doc: "Adds e to the entity list. e.update(e) and e.draw(e) are called if present; e.dead = true removes it on the next update_all(). Errors if e is not a table.",
    },
    ApiEntry {
        name: "Entities.update_all",
        params: &[],
        returns: "nil",
        doc: "Calls e.update(e) on every live entity (if present), then removes any entity with e.dead == true.",
    },
    ApiEntry {
        name: "Entities.draw_all",
        params: &[],
        returns: "nil",
        doc: "Calls e.draw(e) on every live entity (if present), in the order they were added.",
    },
    ApiEntry {
        name: "Entities.clear",
        params: &[],
        returns: "nil",
        doc: "Removes all entities.",
    },
    ApiEntry {
        name: "Entities.count",
        params: &[],
        returns: "number",
        doc: "Number of live entities.",
    },
    ApiEntry {
        name: "Entities.new",
        params: &[],
        returns: "table",
        doc: "Returns a fresh, independent entity list with its own add/update_all/draw_all/clear/count methods, for carts that want one list per scene instead of the shared default list.",
    },
    ApiEntry {
        name: "Camera.follow",
        params: &[param!("entity": "table"), param!("opts": "table?")],
        returns: "nil",
        doc: "Tracks entity's position (entity.pos, a Vec2, or entity.x/entity.y) on every Camera.update() call. opts = { lerp, deadzone_x, deadzone_y }, all optional: lerp defaults to 1 (instant snap), deadzone_x/deadzone_y default to 0 (camera moves on any target movement). Errors immediately if entity has neither .pos nor .x/.y.",
    },
    ApiEntry {
        name: "Camera.unfollow",
        params: &[],
        returns: "nil",
        doc: "Stops following the current target. Camera.update() then holds its last position.",
    },
    ApiEntry {
        name: "Camera.shake",
        params: &[param!("amount": "number"), param!("duration": "number")],
        returns: "nil",
        doc: "Adds random jitter (up to +/- amount per axis) on top of the followed position for duration frames, linearly decaying to 0.",
    },
    ApiEntry {
        name: "Camera.update",
        params: &[],
        returns: "nil",
        doc: "Advances follow smoothing and shake decay by one frame, then calls set_camera() with the result. A no-op position-wise if Camera.follow() was never called. The computed position is clamped to >= 0 before calling set_camera (which takes unsigned coordinates).",
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
        params: &[
            param!("s": "string"),
            param!("i": "number"),
            param!("j?": "number"),
        ],
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
        params: &[
            param!("s": "string"),
            param!("pattern": "string"),
            param!("init?": "number"),
        ],
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
        params: &[
            param!("s": "string"),
            param!("pattern": "string"),
            param!("init?": "number"),
        ],
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
        params: &[
            param!("t": "table"),
            param!("pos?": "number"),
            param!("value": "any"),
        ],
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
        .chain(PRELUDE.iter())
        .chain(STDLIB.iter())
        .find(|e| e.name == name)
}

pub fn all_names() -> impl Iterator<Item = &'static str> {
    BUILTINS
        .iter()
        .chain(PRELUDE.iter())
        .chain(STDLIB.iter())
        .map(|e| e.name)
}

/// `PRELUDE` entries name members (`"Vec2.new"`, `"Camera:shake"`) rather
/// than bare globals; the owning global is whatever precedes the first
/// `.`/`:`.
fn root_identifier(entry_name: &str) -> &str {
    entry_name.split(['.', ':']).next().unwrap_or(entry_name)
}

/// Which opt-in prelude module a `PRELUDE` entry belongs to, or `None` for
/// the always-on core (`lerp`, `clamp`, `ease_*`) or for `BUILTINS`/`STDLIB`
/// entries (meaningless for those, but harmless to call). Used by Studio to
/// scope the API payload and diagnostics to a cart's enabled `[stdlib]`
/// modules.
pub fn prelude_entry_module(entry: &ApiEntry) -> Option<&'static str> {
    let root = root_identifier(entry.name);
    super::lua_exec::prelude_module_globals()
        .into_iter()
        .find(|(_, globals)| globals.contains(&root))
        .map(|(name, _)| name)
}

#[cfg(test)]
mod prelude_consistency_tests {
    use super::*;

    /// Every `PRELUDE` entry must name a global that some current prelude
    /// source (core or a module) actually defines — an entry left behind
    /// after a rename/removal in `lua_exec.rs` would otherwise document a
    /// name that no longer exists.
    #[test]
    fn every_prelude_entry_names_a_global_that_still_exists() {
        let core = super::super::lua_exec::core_prelude_names();
        let modules = super::super::lua_exec::prelude_module_globals();
        for entry in PRELUDE {
            let root = root_identifier(entry.name);
            let known =
                core.contains(&root) || modules.iter().any(|(_, globals)| globals.contains(&root));
            assert!(
                known,
                "PRELUDE entry \"{}\" names global \"{root}\", which no core or \
                 module source in lua_exec::PRELUDE_MODULES defines anymore",
                entry.name
            );
        }
    }

    /// Every global a prelude module (or core) actually defines must have at
    /// least one `PRELUDE` entry — a new/renamed global with no matching
    /// entry would otherwise go undocumented in Studio's autocomplete.
    #[test]
    fn every_prelude_global_has_at_least_one_entry() {
        let core = super::super::lua_exec::core_prelude_names();
        let modules = super::super::lua_exec::prelude_module_globals();
        let documented_roots: std::collections::HashSet<&str> = PRELUDE
            .iter()
            .map(|entry| root_identifier(entry.name))
            .collect();

        for &name in core {
            // Internal bookkeeping flag, not a name a cart is meant to call —
            // deliberately undocumented, unlike every other core global.
            if name == "RTK_SEEDED" {
                continue;
            }
            assert!(
                documented_roots.contains(name),
                "core prelude global \"{name}\" has no PRELUDE entry"
            );
        }
        for (module_name, globals) in &modules {
            for &name in *globals {
                assert!(
                    documented_roots.contains(name),
                    "module \"{module_name}\"'s global \"{name}\" has no PRELUDE entry"
                );
            }
        }
    }

    #[test]
    fn prelude_entry_module_maps_representative_entries() {
        let vec2_new = lookup("Vec2.new").expect("Vec2.new should be a documented entry");
        assert_eq!(prelude_entry_module(vec2_new), Some("vec2"));

        let aabb = lookup("aabb_overlap").expect("aabb_overlap should be a documented entry");
        assert_eq!(prelude_entry_module(aabb), Some("collision"));

        let lerp = lookup("lerp").expect("lerp should be a documented core entry");
        assert_eq!(prelude_entry_module(lerp), None);
    }

    #[test]
    fn every_non_core_prelude_entry_maps_to_some_module() {
        let core = super::super::lua_exec::core_prelude_names();
        for entry in PRELUDE {
            let root = root_identifier(entry.name);
            if core.contains(&root) {
                continue;
            }
            assert!(
                prelude_entry_module(entry).is_some(),
                "PRELUDE entry \"{}\" (root \"{root}\") doesn't map to any opt-in module",
                entry.name
            );
        }
    }
}
