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
            Key::Character(SmolStr::new_inline("q")),
            Keybind {
                name: "keybind_drop_item",
                description: "Keybinding for dropping items",
                action: Actionkey::DropItem,
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
        (
            Key::Character(SmolStr::new_inline("1")),
            Keybind {
                name: "keybind_hotbar_1",
                description: "Keybind for selecting slot 1 of the hotbar",
                action: Actionkey::Hotbar1,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("2")),
            Keybind {
                name: "keybind_hotbar_2",
                description: "Keybind for selecting slot 2 of the hotbar",
                action: Actionkey::Hotbar2,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("3")),
            Keybind {
                name: "keybind_hotbar_3",
                description: "Keybind for selecting slot 3 of the hotbar",
                action: Actionkey::Hotbar3,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("4")),
            Keybind {
                name: "keybind_hotbar_4",
                description: "Keybind for selecting slot 4 of the hotbar",
                action: Actionkey::Hotbar4,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("5")),
            Keybind {
                name: "keybind_hotbar_5",
                description: "Keybind for selecting slot 5 of the hotbar",
                action: Actionkey::Hotbar5,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("6")),
            Keybind {
                name: "keybind_hotbar_6",
                description: "Keybind for selecting slot 6 of the hotbar",
                action: Actionkey::Hotbar6,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("7")),
            Keybind {
                name: "keybind_hotbar_7",
                description: "Keybind for selecting slot 7 of the hotbar",
                action: Actionkey::Hotbar7,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("8")),
            Keybind {
                name: "keybind_hotbar_8",
                description: "Keybind for selecting slot 8 of the hotbar",
                action: Actionkey::Hotbar8,
            },
        ),
        (
            Key::Character(SmolStr::new_inline("9")),
            Keybind {
                name: "keybind_hotbar_9",
                description: "Keybind for selecting slot 9 of the hotbar",
                action: Actionkey::Hotbar9,
            },
        ),
    ]
}
