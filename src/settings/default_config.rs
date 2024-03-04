use super::*;

pub fn default_vars() -> Vec<(SettingType, ConfigVar)> {
    vec![
        (
            SettingType::MaxFps,
            ConfigVar {
                name: "max_fps",
                description: "fps_max caps the maximum FPS for the rendering engine",
                serializable: true,
                value: SettingValue::Num(60),
            },
        ),
        (
            SettingType::FOV,
            ConfigVar {
                name: "fov",
                description: "Setting for controlling the client field of view",
                serializable: true,
                value: SettingValue::Num(90),
            },
        ),
        (
            SettingType::Vsync,
            ConfigVar {
                name: "vsync",
                description: "Toggle to enable/disable vsync",
                serializable: true,
                value: SettingValue::Bool(false),
            },
        ),
        (
            SettingType::MouseSense,
            ConfigVar {
                name: "mouse_sens",
                description: "Mouse Sensitivity",
                serializable: true,
                value: SettingValue::Float(1.0),
            },
        ),
        (
            SettingType::MasterVolume,
            ConfigVar {
                name: "master_volume",
                description: "Main volume control",
                serializable: true,
                value: SettingValue::Num(100),
            },
        ),
        (
            SettingType::CapeVisible,
            ConfigVar {
                name: "cape",
                description: "Toggle your cape",
                serializable: true,
                value: SettingValue::Bool(false),
            },
        ),
        (
            SettingType::JacketVisible,
            ConfigVar {
                name: "jacket",
                description: "Toggle your jacket",
                serializable: true,
                value: SettingValue::Bool(false),
            },
        ),
        (
            SettingType::LeftSleeveVisible,
            ConfigVar {
                name: "left_sleeve",
                description: "Toggle your left sleeve",
                serializable: true,
                value: SettingValue::Bool(false),
            },
        ),
        (
            SettingType::RightSleeveVisible,
            ConfigVar {
                name: "right_sleeve",
                description: "Toggle your right sleeve",
                serializable: true,
                value: SettingValue::Bool(false),
            },
        ),
        (
            SettingType::LeftPantsVisible,
            ConfigVar {
                name: "left_pants",
                description: "Toggle your left pants",
                serializable: true,
                value: SettingValue::Bool(false),
            },
        ),
        (
            SettingType::RightPantsVisible,
            ConfigVar {
                name: "right_pants",
                description: "Toggle your right pants",
                serializable: true,
                value: SettingValue::Bool(false),
            },
        ),
        (
            SettingType::HatVisible,
            ConfigVar {
                name: "hat",
                description: "Toggle your hat",
                serializable: true,
                value: SettingValue::Bool(false),
            },
        ),
        (
            SettingType::AutomaticOfflineAccounts,
            ConfigVar {
                name: "automatic_offline_accounts",
                description:
                    "Enables using no password in the login screen for creating offline accounts",
                serializable: true,
                value: SettingValue::Bool(false),
            },
        ),
        (
            SettingType::LogLevelTerm,
            ConfigVar {
                name: "log_level_term",
                description: "log level of messages to log to the terminal",
                serializable: true,
                value: SettingValue::String("info".to_owned()),
            },
        ),
        (
            SettingType::LogLevelFile,
            ConfigVar {
                name: "log_level_file",
                description: "log level of messages to log to the file",
                serializable: true,
                value: SettingValue::String("trace".to_owned()),
            },
        ),
        (
            SettingType::BackgroundImage,
            ConfigVar {
                name: "background",
                description: "Select the background image",
                serializable: true,
                value: SettingValue::String("leafish:gui/background".to_owned()),
            },
        ),
        (
            SettingType::AuthClientToken,
            ConfigVar {
                name: "auth_client_token",
                description: r#"auth_client_token is a token that stays static between sessions.
Used to identify this client vs others."#,
                serializable: true,
                value: SettingValue::String("".to_owned()),
            },
        ),
    ]
}
