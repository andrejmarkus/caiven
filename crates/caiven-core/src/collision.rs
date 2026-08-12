/// Bitset of engine-affecting collision behaviors: `SOLID`, `ONE_WAY`,
/// `SLOPE_LEFT`, `SLOPE_RIGHT`. A type is meant to be flat-solid, one-way,
/// or exactly one slope direction — mutually exclusive by convention, not
/// enforced here (see `move_and_collide`'s fixed priority order: solid,
/// then one-way, then slope). The representation is a plain `u8` so new
/// bits can be added later without changing the on-disk format (unknown
/// bits round-trip untouched).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CollisionTypeFlags(u8);

impl CollisionTypeFlags {
    pub const SOLID: u8 = 0b0000_0001;
    pub const ONE_WAY: u8 = 0b0000_0010;
    pub const SLOPE_LEFT: u8 = 0b0000_0100;
    pub const SLOPE_RIGHT: u8 = 0b0000_1000;

    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn is_solid(self) -> bool {
        self.0 & Self::SOLID != 0
    }

    pub const fn is_one_way(self) -> bool {
        self.0 & Self::ONE_WAY != 0
    }

    pub const fn is_slope_left(self) -> bool {
        self.0 & Self::SLOPE_LEFT != 0
    }

    pub const fn is_slope_right(self) -> bool {
        self.0 & Self::SLOPE_RIGHT != 0
    }
}

/// One entry of a cart's collision-type table: a user-defined (or built-in)
/// meaning for a raw collision cell value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollisionType {
    pub id: u8,
    pub name: String,
    pub color: [u8; 3],
    pub flags: CollisionTypeFlags,
}

/// Ids reserved for the built-in collision types. Custom types must use
/// ids outside this set (3..=255).
pub const BUILTIN_COLLISION_TYPE_IDS: [u8; 3] = [0, 1, 2];

/// The default collision-type table every new/legacy cart is seeded with.
pub fn builtin_collision_types() -> Vec<CollisionType> {
    vec![
        CollisionType {
            id: 0,
            name: "walkable".to_string(),
            color: [0, 0, 0],
            flags: CollisionTypeFlags::from_bits(0),
        },
        CollisionType {
            id: 1,
            name: "solid".to_string(),
            color: [255, 176, 0],
            flags: CollisionTypeFlags::from_bits(CollisionTypeFlags::SOLID),
        },
        CollisionType {
            id: 2,
            name: "hazard".to_string(),
            color: [224, 32, 32],
            flags: CollisionTypeFlags::from_bits(0),
        },
    ]
}

/// Looks up a collision type by id.
pub fn collision_type_by_id(types: &[CollisionType], id: u8) -> Option<&CollisionType> {
    types.iter().find(|t| t.id == id)
}

/// Looks up a collision type by name (exact match).
pub fn collision_type_by_name<'a>(
    types: &'a [CollisionType],
    name: &str,
) -> Option<&'a CollisionType> {
    types.iter().find(|t| t.name == name)
}

/// True if `id` is defined in `types` and flagged solid. Undefined ids are
/// never solid.
pub fn is_solid_id(types: &[CollisionType], id: u8) -> bool {
    collision_type_by_id(types, id).is_some_and(|t| t.flags.is_solid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_have_reserved_ids_and_expected_solid_flag() {
        let types = builtin_collision_types();
        assert_eq!(types.len(), 3);
        assert!(!is_solid_id(&types, 0));
        assert!(is_solid_id(&types, 1));
        assert!(!is_solid_id(&types, 2));
    }

    #[test]
    fn undefined_id_is_not_solid() {
        let types = builtin_collision_types();
        assert!(!is_solid_id(&types, 42));
    }

    #[test]
    fn lookup_by_name_and_id() {
        let types = builtin_collision_types();
        assert_eq!(
            collision_type_by_name(&types, "hazard")
                .expect("hazard type")
                .id,
            2
        );
        assert_eq!(
            collision_type_by_id(&types, 1).expect("id 1 type").name,
            "solid"
        );
        assert!(collision_type_by_name(&types, "nope").is_none());
    }

    #[test]
    fn flags_roundtrip_unknown_bits() {
        let flags = CollisionTypeFlags::from_bits(0b1000_0011);
        assert!(flags.is_solid());
        assert_eq!(flags.bits(), 0b1000_0011);
    }

    #[test]
    fn one_way_and_slope_flags_are_independently_readable() {
        let one_way = CollisionTypeFlags::from_bits(CollisionTypeFlags::ONE_WAY);
        assert!(one_way.is_one_way());
        assert!(!one_way.is_slope_left());
        assert!(!one_way.is_slope_right());
        assert!(!one_way.is_solid());

        let slope_left = CollisionTypeFlags::from_bits(CollisionTypeFlags::SLOPE_LEFT);
        assert!(slope_left.is_slope_left());
        assert!(!slope_left.is_one_way());
        assert!(!slope_left.is_slope_right());

        let slope_right = CollisionTypeFlags::from_bits(CollisionTypeFlags::SLOPE_RIGHT);
        assert!(slope_right.is_slope_right());
        assert!(!slope_right.is_one_way());
        assert!(!slope_right.is_slope_left());
    }

    #[test]
    fn undefined_flags_default_to_false_for_new_shapes() {
        let flags = CollisionTypeFlags::default();
        assert!(!flags.is_one_way());
        assert!(!flags.is_slope_left());
        assert!(!flags.is_slope_right());
    }
}
