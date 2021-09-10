use crate::settings;
use crate::settings::CVar;
use std::marker::PhantomData;
use winit::event::VirtualKeyCode;

pub const R_MAX_FPS: settings::CVar<i64> = settings::CVar {
    ty: PhantomData,
    name: "r_max_fps",
    description: "fps_max caps the maximum FPS for the rendering engine",
    mutable: true,
    serializable: true,
    default: &|| 60,
};

pub const R_FOV: settings::CVar<i64> = settings::CVar {
    ty: PhantomData,
    name: "r_fov",
    description: "Setting for controlling the client field of view",
    mutable: true,
    serializable: true,
    default: &|| 90,
};

pub const R_VSYNC: settings::CVar<bool> = settings::CVar {
    ty: PhantomData,
    name: "r_vsync",
    description: "Toggle to enable/disable vsync",
    mutable: true,
    serializable: true,
    default: &|| false,
};

pub const CL_MASTER_VOLUME: settings::CVar<i64> = settings::CVar {
    ty: PhantomData,
    name: "cl_master_volume",
    description: "Main volume control",
    mutable: true,
    serializable: true,
    default: &|| 100,
};

// https://github.com/SpigotMC/BungeeCord/blob/bda160562792a913cba3a65ba4996de60d0d6d68/proxy/src/main/java/net/md_5/bungee/PlayerSkinConfiguration.java#L20
pub const S_CAPE: settings::CVar<bool> = settings::CVar {
    //
    ty: PhantomData,
    name: "s_cape",
    description: "Toggle your cape",
    mutable: true,
    serializable: true,
    default: &|| false,
};

pub const S_JACKET: settings::CVar<bool> = settings::CVar {
    //
    ty: PhantomData,
    name: "s_jacket",
    description: "Toggle your jacket",
    mutable: true,
    serializable: true,
    default: &|| false,
};

pub const S_LEFT_SLEEVE: settings::CVar<bool> = settings::CVar {
    //
    ty: PhantomData,
    name: "s_left_sleeve",
    description: "Toggle your left sleeve",
    mutable: true,
    serializable: true,
    default: &|| false,
};

pub const S_RIGHT_SLEEVE: settings::CVar<bool> = settings::CVar {
    //
    ty: PhantomData,
    name: "s_right_sleeve",
    description: "Toggle your right sleeve",
    mutable: true,
    serializable: true,
    default: &|| false,
};

pub const S_LEFT_PANTS: settings::CVar<bool> = settings::CVar {
    //
    ty: PhantomData,
    name: "s_left_pants",
    description: "Toggle your left pants",
    mutable: true,
    serializable: true,
    default: &|| false,
};

pub const S_RIGHT_PANTS: settings::CVar<bool> = settings::CVar {
    //
    ty: PhantomData,
    name: "s_right_pants",
    description: "Toggle your right pants",
    mutable: true,
    serializable: true,
    default: &|| false,
};

pub const S_HAT: settings::CVar<bool> = settings::CVar {
    //
    ty: PhantomData,
    name: "s_hat",
    description: "Toggle your hat",
    mutable: true,
    serializable: true,
    default: &|| false,
};

macro_rules! create_keybind {
    ($keycode:ident, $name:expr, $description:expr) => {
        settings::CVar {
            ty: PhantomData,
            name: $name,
            description: $description,
            mutable: true,
            serializable: true,
            default: &|| VirtualKeyCode::$keycode as i64,
        }
    };
}

pub const CL_KEYBIND_FORWARD: settings::CVar<i64> =
    create_keybind!(W, "cl_keybind_forward", "Keybinding for moving forward");
pub const CL_KEYBIND_BACKWARD: settings::CVar<i64> =
    create_keybind!(S, "cl_keybind_backward", "Keybinding for moving backward");
pub const CL_KEYBIND_LEFT: settings::CVar<i64> =
    create_keybind!(A, "cl_keybind_left", "Keybinding for moving the left");
pub const CL_KEYBIND_RIGHT: settings::CVar<i64> =
    create_keybind!(D, "cl_keybind_right", "Keybinding for moving to the right");
pub const CL_KEYBIND_OPEN_INV: settings::CVar<i64> = create_keybind!(
    E,
    "cl_keybind_open_inv",
    "Keybinding for opening the inventory"
);
pub const CL_KEYBIND_SNEAK: settings::CVar<i64> =
    create_keybind!(LShift, "cl_keybind_sneak", "Keybinding for sneaking");
pub const CL_KEYBIND_SPRINT: settings::CVar<i64> =
    create_keybind!(LControl, "cl_keybind_sprint", "Keybinding for sprinting");
pub const CL_KEYBIND_JUMP: settings::CVar<i64> =
    create_keybind!(Space, "cl_keybind_jump", "Keybinding for jumping");
pub const CL_KEYBIND_TOGGLE_HUD: settings::CVar<i64> = create_keybind!(
    F1,
    "cl_keybind_toggle_hud",
    "Keybinding for toggling the hud"
);
pub const CL_KEYBIND_TOGGLE_DEBUG: settings::CVar<i64> = create_keybind!(
    F3,
    "cl_keybind_toggle_debug",
    "Keybinding for toggling the debug info"
);

pub const BACKGROUND_IMAGE: settings::CVar<String> = CVar {
    ty: PhantomData,
    name: "background",
    description: "Select the background image",
    mutable: true,
    serializable: true,
    default: &|| String::from("leafish:gui/background"),
};

pub const DOUBLE_JUMP_MS: u32 = 100;

pub fn register_vars(vars: &mut settings::Vars) {
    vars.register(R_MAX_FPS);
    vars.register(R_FOV);
    vars.register(R_VSYNC);
    vars.register(CL_MASTER_VOLUME);
    vars.register(CL_KEYBIND_FORWARD);
    vars.register(CL_KEYBIND_BACKWARD);
    vars.register(CL_KEYBIND_LEFT);
    vars.register(CL_KEYBIND_RIGHT);
    vars.register(CL_KEYBIND_OPEN_INV);
    vars.register(CL_KEYBIND_SNEAK);
    vars.register(CL_KEYBIND_SPRINT);
    vars.register(CL_KEYBIND_JUMP);
    vars.register(CL_KEYBIND_TOGGLE_HUD);
    vars.register(CL_KEYBIND_TOGGLE_DEBUG);
    vars.register(S_CAPE);
    vars.register(S_JACKET);
    vars.register(S_LEFT_SLEEVE);
    vars.register(S_RIGHT_SLEEVE);
    vars.register(S_LEFT_PANTS);
    vars.register(S_RIGHT_PANTS);
    vars.register(S_HAT);
    vars.register(BACKGROUND_IMAGE);
}

#[derive(Hash, PartialEq, Eq, Debug, Copy, Clone)]
pub enum Actionkey {
    Forward,
    Backward,
    Left,
    Right,
    OpenInv,
    Sneak,
    Sprint,
    Jump,
    ToggleHud,
    ToggleDebug,
}

impl Actionkey {
    pub fn values() -> Vec<Actionkey> {
        vec![
            Actionkey::Forward,
            Actionkey::Backward,
            Actionkey::Left,
            Actionkey::Right,
            Actionkey::OpenInv,
            Actionkey::Sneak,
            Actionkey::Sprint,
            Actionkey::Jump,
            Actionkey::ToggleHud,
            Actionkey::ToggleDebug,
        ]
    }

    pub fn get_by_keycode(keycode: VirtualKeyCode, vars: &settings::Vars) -> Option<Actionkey> {
        for steven_key in Actionkey::values() {
            if keycode as i64 == *vars.get(steven_key.get_cvar()) {
                return Some(steven_key);
            }
        }
        None
    }

    pub fn get_cvar(&self) -> settings::CVar<i64> {
        match *self {
            Actionkey::Forward => CL_KEYBIND_FORWARD,
            Actionkey::Backward => CL_KEYBIND_BACKWARD,
            Actionkey::Left => CL_KEYBIND_LEFT,
            Actionkey::Right => CL_KEYBIND_RIGHT,
            Actionkey::OpenInv => CL_KEYBIND_OPEN_INV,
            Actionkey::Sneak => CL_KEYBIND_SNEAK,
            Actionkey::Sprint => CL_KEYBIND_SPRINT,
            Actionkey::Jump => CL_KEYBIND_JUMP,
            Actionkey::ToggleHud => CL_KEYBIND_TOGGLE_HUD,
            Actionkey::ToggleDebug => CL_KEYBIND_TOGGLE_DEBUG,
        }
    }
}
