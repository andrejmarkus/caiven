//! Bundled example cartridges: real, playable carts shipped inside the
//! Studio binary so the welcome screen's Examples gallery works offline and
//! with no install-time asset step. "Remixing" one unpacks it into a fresh
//! project directory the user picks, exactly like opening a `.cav` they
//! downloaded — everything (code, sprites, sound) stays fully editable.

use serde::Serialize;

pub struct Example {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub bytes: &'static [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleSummary {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

pub const EXAMPLES: [Example; 6] = [
    Example {
        id: "movement",
        name: "Movement",
        description: "Smallest possible playable cart: one sprite, arrow keys",
        bytes: include_bytes!("../../resources/examples/movement.cav"),
    },
    Example {
        id: "catch",
        name: "Catch",
        description: "A minigame with sound effects and a music bank",
        bytes: include_bytes!("../../resources/examples/catch.cav"),
    },
    Example {
        id: "tiles",
        name: "Tiles",
        description: "A tilemap-driven scene built from the map editor",
        bytes: include_bytes!("../../resources/examples/tiles.cav"),
    },
    Example {
        id: "stdlib-demo",
        name: "Stdlib demo",
        description: "Tour of tweens, particles, and animation from the gameplay stdlib",
        bytes: include_bytes!("../../resources/examples/stdlib_demo.cav"),
    },
    Example {
        id: "scenes-demo",
        name: "Scenes demo",
        description: "Scenes, entities, and camera: a title screen that transitions into play",
        bytes: include_bytes!("../../resources/examples/scenes_demo.cav"),
    },
    Example {
        id: "platformer",
        name: "Platformer",
        description: "8-room precision platformer showcasing the full Lua API and stdlib",
        bytes: include_bytes!("../../resources/examples/platformer.cav"),
    },
];

pub fn find(id: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|example| example.id == id)
}

pub fn summaries() -> Vec<ExampleSummary> {
    EXAMPLES
        .iter()
        .map(|example| ExampleSummary {
            id: example.id,
            name: example.name,
            description: example.description,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::cart;
    use caiven_vm::runtime::ConsoleCore;
    use std::collections::HashSet;

    #[test]
    fn example_ids_are_stable_and_unique() {
        let ids = EXAMPLES
            .iter()
            .map(|example| example.id)
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            ["movement", "catch", "tiles", "stdlib-demo", "scenes-demo"]
        );
        assert_eq!(ids.iter().copied().collect::<HashSet<_>>().len(), ids.len());
    }

    #[test]
    fn every_example_is_a_valid_cart_with_lua_source() {
        for example in &EXAMPLES {
            let cart_path = std::env::temp_dir().join(format!(
                "caiven-example-valid-{}-{}.cav",
                example.id,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::write(&cart_path, example.bytes).expect("write temp cart");

            let cart = caiven_cart::load(&cart_path)
                .unwrap_or_else(|e| panic!("{} failed to load as a cart: {e}", example.id));
            assert!(
                cart.sections
                    .iter()
                    .any(|s| s.kind == caiven_cart::SectionKind::LuaSource),
                "{} has no Lua source section",
                example.id
            );

            std::fs::remove_file(&cart_path).ok();
        }
    }

    #[test]
    fn every_example_compiles_and_loads_into_a_project() {
        for example in &EXAMPLES {
            let cav_path = std::env::temp_dir().join(format!(
                "caiven-example-load-{}-{}.cav",
                example.id,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::write(&cav_path, example.bytes).expect("write temp cart");

            let mut console = ConsoleCore::new().expect("console core");
            console.reset_vm();
            let meta = cart::load_cart(&mut console.vm, &cav_path, &console.input, &console.font)
                .unwrap_or_else(|e| panic!("{} failed to load into VM: {e}", example.id));
            assert!(meta.lua_source.is_some());

            std::fs::remove_file(&cav_path).ok();
        }
    }

    #[test]
    fn unknown_example_is_rejected() {
        assert!(find("not-an-example").is_none());
    }
}
