// Copyright 2016 Matthew Collins
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{fmt, str::FromStr};

use crate::translate;
use crate::translate::*;

use serde::Deserialize;

pub use crate::format::color::*;
use crate::protocol::Error;

const LEGACY_CHAR: char = '§';

#[derive(Debug, Clone)]
pub struct Component {
    pub list: Vec<ComponentType>,
}

impl Component {
    pub fn new(component: ComponentType) -> Self {
        Self {
            list: vec![component],
        }
    }

    // TODO: this should not be need, but on place. I am not even sure it needs it.
    pub fn try_update_with_legacy(&self) -> Self {
        Self {
            list: self
                .list
                .iter()
                .map(|comp| Component::from_legacy_str(&comp.get_text(), comp.get_modifier()).list)
                .flatten()
                .collect(),
        }
    }

    pub fn from_legacy_str(str: &str, modifier: &Modifier) -> Self {
        let mut components = Vec::new();
        if str.contains(LEGACY_CHAR) {
            let mut last = 0;
            let mut current_modifiers = modifier.clone();
            let mut iter = str.char_indices();

            while let Some((i, c)) = iter.next() {
                if c != LEGACY_CHAR {
                    continue;
                }
                let next_char = match iter.next() {
                    Some(next_char) => next_char,
                    None => break,
                };
                let color_char = next_char.1.to_lowercase().next().unwrap();
                let text = str[last..i].to_owned();
                last = next_char.0 + 1;

                components.push(ComponentType::Text {
                    text,
                    modifier: current_modifiers.clone(),
                });

                match color_char {
                    '0' => current_modifiers.color = Color::Black,
                    '1' => current_modifiers.color = Color::DarkBlue,
                    '2' => current_modifiers.color = Color::DarkGreen,
                    '3' => current_modifiers.color = Color::DarkAqua,
                    '4' => current_modifiers.color = Color::DarkRed,
                    '5' => current_modifiers.color = Color::DarkPurple,
                    '6' => current_modifiers.color = Color::Gold,
                    '7' => current_modifiers.color = Color::Gray,
                    '8' => current_modifiers.color = Color::DarkGray,
                    '9' => current_modifiers.color = Color::Blue,
                    'a' => current_modifiers.color = Color::Green,
                    'b' => current_modifiers.color = Color::Aqua,
                    'c' => current_modifiers.color = Color::Red,
                    'd' => current_modifiers.color = Color::LightPurple,
                    'e' => current_modifiers.color = Color::Yellow,
                    'f' => current_modifiers.color = Color::White,
                    'k' => current_modifiers.obfuscated = true,
                    'l' => current_modifiers.bold = true,
                    'm' => current_modifiers.strikethrough = true,
                    'n' => current_modifiers.underlined = true,
                    'o' => current_modifiers.italic = true,
                    'r' => {
                        current_modifiers = Modifier {
                            color: Color::White,
                            bold: false,
                            italic: false,
                            underlined: false,
                            strikethrough: false,
                            obfuscated: false,
                        };
                    }
                    _ => {}
                };
            }
            components.push(ComponentType::Text {
                text: str[last..].to_owned(),
                modifier: current_modifiers,
            });
        } else {
            components.push(ComponentType::Text {
                text: str.to_string(),
                modifier: modifier.clone(),
            });
        }

        Self { list: components }
    }

    pub fn from_str(str: &str) -> Self {
        log::trace!("Raw: {}", str);
        match serde_json::from_str::<ChatSections>(str) {
            Ok(sections) => Component::from_chat_sections(sections, &Modifier::default()),
            // Sometimes mojang sends a literal string, so we should interpret it literally
            Err(error) => {
                log::trace!("Failed error: {}", error);
                Component::from_legacy_str(str, &Modifier::default())
            }
        }
    }

    fn get_text(with: &ComponentData, modifier: &Modifier) -> Self {
        match with {
            ComponentData::Chat(chat) => {
                Component::from_chat(&chat, &modifier.over_write(&chat.get_modifier()))
            }

            ComponentData::Str(str) => Self {
                list: vec![ComponentType::Text {
                    text: str.to_string(),
                    modifier: modifier.clone(),
                }],
            },
        }
    }

    fn get_string_from_extra(extra: &[translate::ComponentData], modifier: &Modifier) -> Self {
        Self {
            list: extra
                .iter()
                .map(|chat_or_string| match chat_or_string {
                    ComponentData::Chat(chat) => {
                        Component::from_chat(
                            chat,
                            &modifier.over_write(&chat_or_string.get_modifier()),
                        )
                        .list
                    }
                    ComponentData::Str(str) => {
                        Component::from_legacy_str(
                            str,
                            &modifier.over_write(&chat_or_string.get_modifier()),
                        )
                        .list
                    }
                })
                .flatten()
                .collect::<_>(),
        }
    }

    fn from_chat_sections(sections: ChatSections, modifier: &Modifier) -> Self {
        Component {
            list: sections
                .sections
                .into_iter()
                .flat_map(|c| Self::from_chat(&c, modifier).list)
                .collect(),
        }
    }

    fn from_chat(chat: &Chat, modifier: &Modifier) -> Self {
        let modifier = modifier.over_write(&chat.get_modifier());
        let text_components: Vec<ComponentType> = match (&chat.translate, &chat.text, &chat.extra) {
            (None, None, None) => chat
                .with
                .iter()
                .map(|with| Component::get_text(with, &modifier).list)
                .flatten()
                .collect(),

            (Some(translate), None, None) => {
                let mut list = chat
                    .with
                    .iter()
                    .map(|inner_chat| Component::get_text(inner_chat, &modifier))
                    .collect::<Vec<Component>>();

                let mut iter_component = list.iter_mut();

                let mut components = Vec::new();
                let translated = translate::translate(translate);
                let mut index = 0;
                for (i, char) in translated.char_indices() {
                    match char {
                        '{' => {
                            components.push(ComponentType::Text {
                                text: translated[index..i].to_string(),
                                modifier: modifier.clone(),
                            });
                            match iter_component.next() {
                                Some(component) => components.append(&mut component.list),
                                None => {}
                            };
                        }
                        '}' => index = i + 1,
                        _ => {}
                    }
                }
                components.push(ComponentType::Text {
                    text: translated[index..].to_string(),
                    modifier: modifier.clone(),
                });
                components
            }
            (Some(translate), Some(text), None) => {
                format!("ERR trans: {}, text: {}", translate, text);
                todo!()
            }
            (Some(translate), Some(text), Some(extra)) => {
                format!(
                    "ERR trans: {}, text: {}, extra{:?}, ",
                    translate, text, extra
                );
                todo!()
            }
            (Some(text), None, Some(extra)) => {
                format!("ERR trans: {}, extra: {:?}", text, extra);
                todo!()
            }
            (None, None, Some(extra)) => {
                format!("ERR extra: {:?}", extra);
                todo!()
            }
            (None, Some(text), Some(extra)) => {
                let mut component = Component::from_legacy_str(text, &modifier).list;
                component.append(&mut Component::get_string_from_extra(extra, &modifier).list);
                component
            }
            (None, Some(text), None) => Component::from_legacy_str(text, &modifier).list,
        };
        // chat.build_component_from_string(return_type);
        Component {
            list: text_components,
        }
    }

    pub fn from_json(v: &serde_json::Value) -> Result<Self, Error> {
        match serde_json::from_value::<ChatSections>(v.clone()) {
            Ok(sections) => {
                return Ok(Component::from_chat_sections(
                    sections,
                    &Modifier::default(),
                ))
            }
            // Sometimes mojang sends a literal string, so we should interpret it literally
            Err(error) => {
                log::trace!("Failed error: {}", error);
                Ok(Component::from_legacy_str(
                    match v.as_str() {
                        Some(val) => val,
                        None => return Err(Error::Err(format!("Couldn't parse json: {}", v))),
                    },
                    &Modifier::default(),
                ))
            }
        }
    }

    pub fn to_value(&self) -> serde_json::Value {
        unimplemented!()
    }
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.list
                .iter()
                .map(|comp| comp.get_text())
                .collect::<String>()
        )
    }
}

impl Default for Component {
    fn default() -> Self {
        Component::new(ComponentType::Text {
            text: "".to_owned(),
            modifier: Default::default(),
        })
    }
}

#[derive(Default, Clone)]
// TODO: Use all of the modifiers for rendering
pub struct Modifier {
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikethrough: bool,
    pub obfuscated: bool,
    pub color: Color,
}

impl fmt::Debug for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_struct("Modifier");
        if self.color != Color::None {
            dbg.field("color", &self.color);
        }
        if self.bold {
            dbg.field("bold", &self.bold);
        }
        if self.italic {
            dbg.field("italic", &self.italic);
        }
        if self.underlined {
            dbg.field("underlined", &self.underlined);
        }
        if self.strikethrough {
            dbg.field("strikethrough", &self.strikethrough);
        }
        if self.obfuscated {
            dbg.field("obfuscated", &self.obfuscated);
        }
        dbg.finish_non_exhaustive()
    }
}

// TODO: Missing events click/hover/insert

impl Modifier {
    pub fn to_value(&self) -> serde_json::Value {
        unimplemented!()
    }

    pub fn over_write(&self, modifier: &Self) -> Self {
        Self {
            bold: if modifier.bold { true } else { self.bold },
            italic: if modifier.italic { true } else { self.italic },
            underlined: if modifier.underlined {
                true
            } else {
                self.underlined
            },
            strikethrough: if modifier.strikethrough {
                true
            } else {
                self.strikethrough
            },
            obfuscated: if modifier.obfuscated {
                true
            } else {
                self.obfuscated
            },
            color: if modifier.color != Color::None {
                modifier.color
            } else {
                self.color
            },
        }
    }
}

impl fmt::Display for ComponentType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Ok(write!(
            f,
            "{}",
            match self {
                ComponentType::Text { text, .. } => text,
                ComponentType::Hover { text, .. } => text,
                ComponentType::Click { text, .. } => text,
                ComponentType::ClickAndHover { text, .. } => text,
            }
        )?)
    }
}

#[derive(Debug, Clone)]
pub enum ComponentType {
    Text { text: String, modifier: Modifier },
    // TODO: Implment the rest!
    Hover { text: String, modifier: Modifier },
    Click { text: String, modifier: Modifier },
    ClickAndHover { text: String, modifier: Modifier },
}

impl ComponentType {
    pub fn new(val: &str, color: Option<Color>) -> Self {
        Self::Text {
            text: val.to_string(),
            modifier: match color {
                Some(color) => Modifier {
                    color,
                    ..Modifier::default()
                },
                None => Modifier::default(),
            },
        }
    }

    pub fn from_value(v: &serde_json::Value, modifier: Modifier) -> Self {
        Self::Text {
            text: v.as_str().unwrap_or("").to_owned(),
            modifier,
        }
    }

    pub fn get_text(&self) -> &str {
        match self {
            ComponentType::Text { text, .. } => text.as_str(),
            ComponentType::Hover { text, .. } => text.as_str(),
            ComponentType::Click { text, .. } => text.as_str(),
            ComponentType::ClickAndHover { text, .. } => text.as_str(),
        }
    }

    pub fn get_modifier(&self) -> &Modifier {
        match self {
            ComponentType::Text { modifier, .. } => &modifier,
            ComponentType::Hover { modifier, .. } => &modifier,
            ComponentType::Click { modifier, .. } => &modifier,
            ComponentType::ClickAndHover { modifier, .. } => &modifier,
        }
    }

    pub fn to_value(&self) -> serde_json::Value {
        unimplemented!()
    }
}

impl Chat {
    fn get_modifier(&self) -> Modifier {
        Modifier {
            bold: self.bold.unwrap_or_default(),
            italic: self.italic.unwrap_or_default(),
            underlined: self.underlined.unwrap_or_default(),
            strikethrough: self.strikethrough.unwrap_or_default(),
            obfuscated: self.obfuscated.unwrap_or_default(),
            color: self.color.unwrap_or_default(),
        }
    }

    pub fn build_component_from_string(&self, str_format: String) -> Component {
        Component::new(ComponentType::Text {
            text: str_format,
            modifier: self.get_modifier(),
        })
    }
}

impl ComponentData {
    fn get_modifier(&self) -> Modifier {
        match self {
            ComponentData::Chat(chat) => chat.get_modifier(),
            ComponentData::Str(_) => Modifier::default(),
        }
    }
}

pub mod color {
    use crate::format::*;

    #[derive(PartialEq, Debug)]
    pub enum ParseColorError {
        InvalidLenError(usize),
        ParseIntError(std::num::ParseIntError),
    }
    impl fmt::Display for ParseColorError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ParseColorError::InvalidLenError(len) => {
                    format!(
                        "Parse error str to long or to short, Len: {}, must be at 6",
                        len
                    )
                }
                ParseColorError::ParseIntError(e) => format!("Color parse int error: {}", e),
            }
            .fmt(f)
        }
    }

    impl From<std::num::ParseIntError> for ParseColorError {
        fn from(e: std::num::ParseIntError) -> Self {
            ParseColorError::ParseIntError(e)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Color {
        Black,
        DarkBlue,
        DarkGreen,
        DarkAqua,
        DarkRed,
        DarkPurple,
        Gold,
        Gray,
        DarkGray,
        Blue,
        Green,
        Aqua,
        Red,
        LightPurple,
        Yellow,
        White,
        Reset,
        RGB(RGB),
        None,
    }

    impl Default for Color {
        fn default() -> Self {
            Color::None
        }
    }

    impl<'de> Deserialize<'de> for Color {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let color: String = Deserialize::deserialize(deserializer)?;
            match Color::from_str(&color) {
                Ok(color) => Ok(color),
                Err(e) => Err(serde::de::Error::custom(format!(
                    "Failed to deserialize color: {}",
                    e
                ))),
            }
        }
    }

    impl fmt::Display for Color {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    Color::Black => "black".to_owned(),
                    Color::DarkBlue => "dark_blue".to_owned(),
                    Color::DarkGreen => "dark_green".to_owned(),
                    Color::DarkAqua => "dark_aqua".to_owned(),
                    Color::DarkRed => "dark_red".to_owned(),
                    Color::DarkPurple => "dark_purple".to_owned(),
                    Color::Gold => "gold".to_owned(),
                    Color::Gray => "gray".to_owned(),
                    Color::DarkGray => "dark_gray".to_owned(),
                    Color::Blue => "blue".to_owned(),
                    Color::Green => "green".to_owned(),
                    Color::Aqua => "aqua".to_owned(),
                    Color::Red => "red".to_owned(),
                    Color::LightPurple => "light_purple".to_owned(),
                    Color::Yellow => "yellow".to_owned(),
                    Color::White => "white".to_owned(),
                    Color::Reset => "white".to_owned(),
                    Color::None => "white".to_owned(),
                    Color::RGB(rgb) => format!("#{:02X}{:02X}{:02X}", rgb.red, rgb.green, rgb.blue),
                }
            )
        }
    }

    impl FromStr for Color {
        type Err = ParseColorError;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s.to_lowercase().as_str() {
                "black" => Ok(Color::Black),
                "dark_blue" => Ok(Color::DarkBlue),
                "dark_green" => Ok(Color::DarkGreen),
                "dark_aqua" => Ok(Color::DarkAqua),
                "dark_red" => Ok(Color::DarkRed),
                "dark_purple" => Ok(Color::DarkPurple),
                "gold" => Ok(Color::Gold),
                "gray" => Ok(Color::Gray),
                "dark_gray" => Ok(Color::DarkGray),
                "blue" => Ok(Color::Blue),
                "green" => Ok(Color::Green),
                "aqua" => Ok(Color::Aqua),
                "red" => Ok(Color::Red),
                "light_purple" => Ok(Color::LightPurple),
                "yellow" => Ok(Color::Yellow),
                "white" => Ok(Color::White),
                "reset" => Ok(Color::White),
                s => Ok(Color::RGB(RGB::from_str(s)?)),
            }
        }
    }

    impl Color {
        pub fn to_rgb(&self) -> (u8, u8, u8) {
            match self {
                Color::Black => (0, 0, 0),
                Color::DarkBlue => (0, 0, 170),
                Color::DarkGreen => (0, 170, 0),
                Color::DarkAqua => (0, 170, 170),
                Color::DarkRed => (170, 0, 0),
                Color::DarkPurple => (170, 0, 170),
                Color::Gold => (255, 170, 0),
                Color::Gray => (170, 170, 170),
                Color::DarkGray => (85, 85, 85),
                Color::Blue => (85, 85, 255),
                Color::Green => (85, 255, 85),
                Color::Aqua => (85, 255, 255),
                Color::Red => (255, 85, 85),
                Color::LightPurple => (255, 85, 255),
                Color::Yellow => (255, 255, 85),
                Color::White => (255, 255, 255),
                Color::Reset => (255, 255, 255),
                Color::None => (0, 255, 255),
                Color::RGB(c) => (c.red, c.green, c.blue),
            }
        }

        pub fn use_or_def(self, color: Color) -> Self {
            if self == Color::None {
                color
            } else {
                self
            }
        }
    }

    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub struct RGB {
        pub red: u8,
        pub green: u8,
        pub blue: u8,
    }

    impl RGB {
        pub fn new(red: u8, green: u8, blue: u8) -> Self {
            RGB { red, green, blue }
        }
    }

    impl FromStr for RGB {
        type Err = ParseColorError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let without_prefix = s.trim_start_matches("#");
            if without_prefix.len() != 6 {
                return Err(ParseColorError::InvalidLenError(without_prefix.len()));
            }

            let red = u8::from_str_radix(&without_prefix[0..2], 16)?;
            let green = u8::from_str_radix(&without_prefix[2..4], 16)?;
            let blue = u8::from_str_radix(&without_prefix[4..6], 16)?;
            Ok(RGB { red, green, blue })
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn chat() {
        assert_eq!(
            serde_json::from_str::<Chat>(
                r#"{
                    "unknownField": "bar"
                }"#
            )
            .unwrap(),
            Chat::default()
        );
        assert_eq!(
            serde_json::from_str::<Chat>(
                r#"{
                    "text": "hello world!",
                    "translate": "foo.bar",
                    "color": "green",
                    "bold": true,
                    "italic": false,
                    "underlined": true,
                    "strikethrough": true,
                    "obfuscated": false,
                    "clickEvent": {
                        "action": "foo",
                        "data": "Hello world!",
                    },
                    "hoverEvent": {
                        "action": "foo",
                        "data": "Hello world!",
                    },
                    "insertion": "",
                    "extra": [],
                    "with": []
                }"#
            )
            .unwrap(),
            Chat {
                text: Some("hello world!".into()),
                ..Chat::default()
            }
        );
    }

    #[test]
    fn chat_section() {
        assert_eq!(
            serde_json::from_str::<ChatSections>("[]").unwrap(),
            ChatSections { sections: vec![] }
        );
        assert_eq!(
            serde_json::from_str::<ChatSections>(r#"{"text":"hello world!"}"#).unwrap(),
            ChatSections {
                sections: vec![Chat {
                    text: Some("hello world!".into()),
                    ..Chat::default()
                }]
            }
        );
        assert_eq!(
            serde_json::from_str::<ChatSections>(r#"[{"text":"foo"},{"text":"bar"}]"#).unwrap(),
            ChatSections {
                sections: vec![
                    Chat {
                        text: Some("foo".into()),
                        ..Chat::default()
                    },
                    Chat {
                        text: Some("bar".into()),
                        ..Chat::default()
                    }
                ]
            }
        );
    }

    #[test]
    fn test_color_from() {
        match Color::from_str("FF0000").expect("could not parse FF0000") {
            Color::RGB(rgb) => assert_eq!(
                rgb,
                RGB {
                    red: 255,
                    green: 0,
                    blue: 0
                }
            ),
            _ => panic!("Could not parse hex color correct"),
        }
        match Color::from_str("#00FF00").expect("could not parse #00FF00") {
            Color::RGB(rgb) => assert_eq!(
                rgb,
                RGB {
                    red: 0,
                    green: 255,
                    blue: 0
                }
            ),
            _ => panic!("Could not parse hex color correct"),
        }
        match Color::from_str("") {
            Ok(_) => {}
            Err(e) => assert_eq!(ParseColorError::InvalidLenError(0), e),
        }
        match Color::from_str("4343433") {
            Ok(_) => {}
            Err(e) => assert_eq!(ParseColorError::InvalidLenError(7), e),
        }
        match Color::from_str("#4343433") {
            Ok(_) => {}
            Err(e) => assert_eq!(ParseColorError::InvalidLenError(7), e),
        }
        match Color::from_str("#123456").expect("could not parse #123456") {
            Color::RGB(rgb) => assert_eq!(
                rgb,
                RGB {
                    red: 0x12,
                    green: 0x34,
                    blue: 0x56,
                }
            ),
            _ => panic!("Could not parse hex color correct"),
        }

        match Color::from_str("red") {
            Ok(Color::Red) => {}
            _ => panic!("Wrong type"),
        }
        match Color::from_str("BLUE") {
            Ok(Color::Blue) => {}
            _ => panic!("Wrong type"),
        }
        match Color::from_str("dark_blue") {
            Ok(Color::DarkBlue) => {}
            _ => panic!("Wrong type"),
        }
    }
}
