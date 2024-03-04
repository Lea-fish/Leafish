use winit::keyboard::KeyCode;

use super::*;

pub fn create_keybinds() -> Vec<(KeyCode, Keybind)> {
    vec![
        (
            KeyCode::KeyW,
            Keybind {
                name: "keybind_forward",
                description: "Keybinding for moving forward",
                action: Actionkey::Forward,
            },
        ),
        (
            KeyCode::KeyS,
            Keybind {
                name: "keybind_backward",
                description: "Keybinding for moving backward",
                action: Actionkey::Backward,
            },
        ),
        (
            KeyCode::KeyA,
            Keybind {
                name: "keybind_left",
                description: "Keybinding for moving to the left",
                action: Actionkey::Left,
            },
        ),
        (
            KeyCode::KeyD,
            Keybind {
                name: "keybind_right",
                description: "Keybinding for moving to the right",
                action: Actionkey::Right,
            },
        ),
        (
            KeyCode::KeyE,
            Keybind {
                name: "keybind_open_inv",
                description: "Keybinding for opening the inventory",
                action: Actionkey::OpenInv,
            },
        ),
        (
            KeyCode::ShiftLeft,
            Keybind {
                name: "keybind_sneak",
                description: "Keybinding for sneaking",
                action: Actionkey::Sneak,
            },
        ),
        (
            KeyCode::ControlLeft,
            Keybind {
                name: "keybind_sprint",
                description: "Keybinding for sprinting",
                action: Actionkey::Sprint,
            },
        ),
        (
            KeyCode::Space,
            Keybind {
                name: "keybind_jump",
                description: "Keybinding for jumping",
                action: Actionkey::Jump,
            },
        ),
        (
            KeyCode::F1,
            Keybind {
                name: "keybind_toggle_hud",
                description: "Keybinding for toggeling the hud",
                action: Actionkey::ToggleHud,
            },
        ),
        (
            KeyCode::F3,
            Keybind {
                name: "keybind_toggle_debug_info",
                description: "Keybinding for toggeling the debug info",
                action: Actionkey::ToggleDebug,
            },
        ),
        (
            KeyCode::KeyT,
            Keybind {
                name: "keybind_toggle_chat",
                description: "Keybinding for toggeling the chat",
                action: Actionkey::ToggleChat,
            },
        ),
    ]
}
