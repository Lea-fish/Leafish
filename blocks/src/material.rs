pub struct Material {
    pub renderable: bool,
    pub should_cull_against: bool,
    pub never_cull: bool, // Because leaves suck
    pub force_shade: bool,
    pub transparent: bool,
    pub absorbed_light: u8,
    pub emitted_light: u8,
    pub collidable: bool,
}

pub const INVISIBLE: Material = Material {
    renderable: false,
    never_cull: false,
    should_cull_against: false,
    force_shade: false,
    transparent: false,
    absorbed_light: 0, // Special because of sky light
    emitted_light: 0,
    collidable: true,
};

pub const INTERACTABLE: Material = Material {
    never_cull: false,
    should_cull_against: false,
    force_shade: false,
    transparent: false,
    collidable: false,
    ..INVISIBLE
};

pub const SOLID: Material = Material {
    renderable: true,
    never_cull: false,
    should_cull_against: true,
    force_shade: false,
    transparent: false,
    absorbed_light: 15,
    emitted_light: 0,
    collidable: true,
};

pub const PARTIALLY_SOLID: Material = Material {
    collidable: true,
    ..NON_SOLID
};

pub const NON_SOLID: Material = Material {
    should_cull_against: false,
    collidable: false,
    absorbed_light: 1,
    ..SOLID
};

pub const TRANSPARENT: Material = Material {
    transparent: true,
    ..PARTIALLY_SOLID
};

pub const LEAVES: Material = Material {
    never_cull: true,
    force_shade: true,
    ..PARTIALLY_SOLID
};
