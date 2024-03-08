use winit::keyboard::{Key, NamedKey, SmolStr};

use super::*;

pub fn create_keybinds() -> Vec<(Key, Keybind)> {
    vec![
        (
            Key::Character(SmolStr::new_inline("w")),
            Keybind {
                name: "keybind_forward",
                description: "Keybinding for moving forward",
                action: Actionkey::Forward,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("s")),
            Keybind {
                name: "keybind_backward",
                description: "Keybinding for moving backward",
                action: Actionkey::Backward,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("a")),
            Keybind {
                name: "keybind_left",
                description: "Keybinding for moving to the left",
                action: Actionkey::Left,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("d")),
            Keybind {
                name: "keybind_right",
                description: "Keybinding for moving to the right",
                action: Actionkey::Right,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("e")),
            Keybind {
                name: "keybind_open_inv",
                description: "Keybinding for opening the inventory",
                action: Actionkey::OpenInv,
            },
        ),
        (
            Key::Named(NamedKey::Shift),
            Keybind {
                name: "keybind_sneak",
                description: "Keybinding for sneaking",
                action: Actionkey::Sneak,
            },
        ),
        (
            Key::Named(NamedKey::Control),
            Keybind {
                name: "keybind_sprint",
                description: "Keybinding for sprinting",
                action: Actionkey::Sprint,
            },
        ),
        (
            Key::Named(NamedKey::Space),
            Keybind {
                name: "keybind_jump",
                description: "Keybinding for jumping",
                action: Actionkey::Jump,
            },
        ),
        (
            Key::Named(NamedKey::F1),
            Keybind {
                name: "keybind_toggle_hud",
                description: "Keybinding for toggeling the hud",
                action: Actionkey::ToggleHud,
            },
        ),
        (
            Key::Named(NamedKey::F3),
            Keybind {
                name: "keybind_toggle_debug_info",
                description: "Keybinding for toggeling the debug info",
                action: Actionkey::ToggleDebug,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("t")),
            Keybind {
                name: "keybind_toggle_chat",
                description: "Keybinding for toggeling the chat",
                action: Actionkey::ToggleChat,
            },
        ),
    ]
}
