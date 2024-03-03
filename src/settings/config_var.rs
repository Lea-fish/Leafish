#[derive(Clone)]
pub enum SettingValue {
    String(String),
    Num(i32),
    Float(f64),
    Bool(bool),
}

#[derive(Clone)]
pub struct ConfigVar {
    pub name: &'static str,
    pub description: &'static str,
    pub serializable: bool,
    pub value: SettingValue,
}

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

#[derive(PartialEq, PartialOrd, Hash, Eq, Ord, Clone, Copy)]
pub enum SettingType {
    MaxFps,
    FOV,
    Vsync,
    MouseSense,
    MasterVolume,
    CapeVisible,
    JacketVisible,
    RightSleeveVisible,
    LeftSleeveVisible,
    RightPantsVisible,
    LeftPantsVisible,
    HatVisible,
    AutomaticOfflineAccounts,
    AuthClientToken,
    BackgroundImage,
    LogLevelFile,
    LogLevelTerm,
}

#[rustfmt::skip]
impl SettingValue {
    pub fn as_string(&self) -> Option<String> {
        if let Self::String(s) = self { Some(s.clone()) } else { None }
    }
    pub fn as_i32(&self) -> Option<i32> {
        if let Self::Num(n) = self { Some(*n) } else { None }
    }
    pub fn as_float(&self) -> Option<f64> {
        if let Self::Float(f) = self { Some(*f) } else { None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(b) = self { Some(*b) } else { None }
    }
}

#[rustfmt::skip]
impl ConfigVar {
    pub fn as_string(&self) -> Option<String> {
        if let SettingValue::String(s) = &self.value { Some(s.to_owned()) } else { None }
    }
    pub fn as_i32(&self) -> Option<i32> {
        if let SettingValue::Num(n) = self.value { Some(n) } else { None }
    }
    pub fn as_float(&self) -> Option<f64> {
        if let SettingValue::Float(f) = self.value { Some(f) } else { None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let SettingValue::Bool(b) = self.value { Some(b) } else { None }
    }
}
