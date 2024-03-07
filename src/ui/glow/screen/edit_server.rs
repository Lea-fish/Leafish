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

use std::collections::BTreeMap;
use std::fs;

use crate::paths;

use crate::ui::glow::ctx;
use crate::ui::glow::render::Renderer;
use crate::ui::glow::ui::logo::Logo;
use crate::ui::glow::ui::ButtonBuilder;
use crate::ui::glow::ui::ButtonRef;
use crate::ui::glow::ui::Container;
use crate::ui::glow::ui::HAttach;
use crate::ui::glow::ui::TextBox;
use crate::ui::glow::ui::TextBoxBuilder;
use crate::ui::glow::ui::TextBoxRef;
use crate::ui::glow::ui::TextBuilder;
use crate::ui::glow::ui::VAttach;
use serde_json::Value;
use std::sync::Arc;

use super::Screen;
use super::ScreenSystem;

pub struct EditServerEntry {
    elements: Option<UIElements>,
    entry_info: Option<(usize, String, String)>,
}

impl Clone for EditServerEntry {
    fn clone(&self) -> Self {
        EditServerEntry {
            elements: None,
            entry_info: self.entry_info.clone(),
        }
    }
}

struct UIElements {
    logo: Logo,

    _name: TextBoxRef,
    _address: TextBoxRef,
    _done: ButtonRef,
    _cancel: ButtonRef,
}

impl EditServerEntry {
    pub fn new(entry_info: Option<(usize, String, String)>) -> EditServerEntry {
        EditServerEntry {
            elements: None,
            entry_info,
        }
    }

    fn save_servers(index: Option<usize>, name: &str, address: &str) {
        let mut servers_info = match fs::File::open(paths::get_data_dir().join("servers.json")) {
            Ok(val) => serde_json::from_reader(val).unwrap(),
            Err(_) => {
                let mut info = BTreeMap::default();
                info.insert("servers".to_owned(), Value::Array(vec![]));
                Value::Object(info.into_iter().collect())
            }
        };

        let new_entry = {
            let mut entry = BTreeMap::default();
            entry.insert("name".to_owned(), Value::String(name.to_owned()));
            entry.insert("address".to_owned(), Value::String(address.to_owned()));
            Value::Object(entry.into_iter().collect())
        };

        {
            let servers = servers_info
                .as_object_mut()
                .unwrap()
                .get_mut("servers")
                .unwrap()
                .as_array_mut()
                .unwrap();
            if let Some(index) = index {
                *servers.get_mut(index).unwrap() = new_entry;
            } else {
                servers.push(new_entry);
            }
        }

        let mut out = fs::File::create(paths::get_data_dir().join("servers.json")).unwrap();
        serde_json::to_writer_pretty(&mut out, &servers_info).unwrap();
    }
}

impl super::Screen for EditServerEntry {
    fn on_active(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        let logo = Logo::new(renderer.resources.clone(), ui_container);

        // Name
        let server_name = TextBoxBuilder::new()
            .input(self.entry_info.as_ref().map_or("", |v| &v.1))
            .position(0.0, -20.0)
            .size(400.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        TextBox::make_focusable(&server_name, ui_container);
        TextBuilder::new()
            .text("Name:")
            .position(0.0, -18.0)
            .attach(&mut *server_name.borrow_mut());

        // Address
        let server_address = TextBoxBuilder::new()
            .input(self.entry_info.as_ref().map_or("", |v| &v.2))
            .position(0.0, 40.0)
            .size(400.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        TextBox::make_focusable(&server_address, ui_container);
        TextBuilder::new()
            .text("Address:")
            .position(0.0, -18.0)
            .attach(&mut *server_address.borrow_mut());

        let save_server_error = TextBuilder::new()
            .text("")
            .position(0.0, 150.0)
            .colour((255, 50, 50, 255))
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);

        // Done
        let done = ButtonBuilder::new()
            .position(110.0, 100.0)
            .size(200.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut done = done.borrow_mut();
            let txt = TextBuilder::new()
                .text("Done")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *done);
            done.add_text(txt);
            let index = self.entry_info.as_ref().map(|v| v.0);
            let server_name = server_name.clone();
            let server_address = server_address.clone();
            done.add_click_func(move |_, _game| {
                if server_address.borrow().input.is_empty() {
                    save_server_error.borrow_mut().text = "Please enter a Server Address".into();
                    return false;
                }
                Self::save_servers(
                    index,
                    &server_name.borrow().input,
                    &server_address.borrow().input,
                );
                ctx().screen_sys
                    .clone()
                    .replace_screen(Box::new(super::ServerList::new(None)));
                true
            });
        }

        // Cancel
        let cancel = ButtonBuilder::new()
            .position(-110.0, 100.0)
            .size(200.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut cancel = cancel.borrow_mut();
            let txt = TextBuilder::new()
                .text("Cancel")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *cancel);
            cancel.add_text(txt);
            cancel.add_click_func(|_, _game| {
                ctx().screen_sys
                    .clone()
                    .replace_screen(Box::new(super::ServerList::new(None)));
                true
            });
        }

        self.elements = Some(UIElements {
            logo,
            _name: server_name,
            _address: server_address,
            _done: done,
            _cancel: cancel,
        });
    }

    fn on_deactive(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
        // Clean up
        self.elements = None
    }

    fn tick(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
        _delta: f64,
    ) {
        let elements = self.elements.as_mut().unwrap();
        elements.logo.tick(renderer);
    }

    fn is_closable(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}
