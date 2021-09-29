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

use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

use crate::format;
use crate::format::{Component, TextComponent};
use crate::paths;
use crate::protocol;
use crate::render;
use crate::settings;
use crate::ui;

use crate::render::hud::{Hud, HudContext};
use crate::render::Renderer;
use crate::screen::Screen;
use crate::ui::Container;
use crossbeam_channel::unbounded;
use crossbeam_channel::{Receiver, TryRecvError};
use instant::Duration;
use parking_lot::RwLock;
use rand::Rng;
use leafish_protocol::protocol::UUID;
use serde::{Serialize, Deserialize};

/// SAFETY: We don't alter components which, which aren't thread safe on other threads than the main one.
unsafe impl Send for Launcher {}
unsafe impl Sync for Launcher {}

pub struct Launcher {
    rendered_accounts: Vec<RenderAccount>,
    options: Option<ui::ButtonRef>,
    background: Option<ui::ImageRef>,
    background_image: String,
    disclaimer: Option<ui::TextRef>,
    accounts: Vec<Account>,
}

impl Clone for Launcher {
    fn clone(&self) -> Self {
        Launcher::new(self.background_image.clone(), self.accounts.to_vec())
    }
}

struct RenderAccount {

    head_picture: Option<ui::ImageRef>,
    account_name: Option<ui::TextRef>,

}

#[derive(Serialize, Deserialize)]
pub struct Account {
    name: String,
    uuid: Option<UUID>,
    verification_tokens: Vec<String>, // this represents the verification tokens used to verify the account, such as hashed passwords, actual tokens, etc
}

impl Clone for Account {
    fn clone(&self) -> Self {
        Account {
            name: self.name.clone(),
            uuid: self.uuid.clone(),
            verification_tokens: self.verification_tokens.to_vec(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum AccountType {

    Mojang,
    Microsoft,
    Custom(String), // Not implemented yet, this will enable us to support other auth services without implementing every single one specifically
    None, // aka. unverified or "offline account" (for offline mode servers)

}

impl Launcher {
    pub fn new(background_image: String, accounts: Vec<Account>) -> Self {
        Launcher {
            rendered_accounts: vec![],
            options: None,
            background: None,
            background_image,
            disclaimer: None,
            accounts,
        }
    }

    /*
    fn init_list(&mut self, renderer: &mut render::Renderer, ui_container: &mut ui::Container) {
        let logo = ui::logo::Logo::new(renderer.resources.clone(), ui_container);

        // Refresh the server list
        let refresh = ui::ButtonBuilder::new()
            .position(300.0, -50.0 - 15.0)
            .size(100.0, 30.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .draw_index(2)
            .create(ui_container);
        {
            let mut refresh = refresh.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Refresh")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *refresh);
            refresh.add_text(txt);
            let nr = self.needs_reload.clone();
            refresh.add_click_func(move |_, _| {
                *nr.borrow_mut() = true;
                true
            })
        }

        // Add a new server to the list
        let add = ui::ButtonBuilder::new()
            .position(200.0, -50.0 - 15.0)
            .size(100.0, 30.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .draw_index(2)
            .create(ui_container);
        {
            let mut add = add.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Add")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *add);
            add.add_text(txt);
            add.add_click_func(move |_, game| {
                game.screen_sys
                    .clone()
                    .replace_screen(Box::new(super::edit_server::EditServerEntry::new(None)));
                true
            })
        }

        // Options menu
        let options = ui::ButtonBuilder::new()
            .position(5.0, 25.0)
            .size(40.0, 40.0)
            .draw_index(1)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Right)
            .create(ui_container);
        {
            let mut options = options.borrow_mut();
            ui::ImageBuilder::new()
                .texture("leafish:gui/cog")
                .position(0.0, 0.0)
                .size(40.0, 40.0)
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *options);
            options.add_click_func(|_, game| {
                game.screen_sys
                    .clone()
                    .add_screen(Box::new(super::SettingsMenu::new(game.vars.clone(), false)));
                true
            });
        }

        // Disclaimer
        let disclaimer = ui::TextBuilder::new()
            .text("Not affiliated with Mojang/Minecraft")
            .position(5.0, 5.0)
            .colour((255, 200, 200, 255))
            .draw_index(1)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Right)
            .create(ui_container);

        // If we are kicked from a server display the reason
        let disconnected = if let Some(ref disconnect_reason) = self.disconnect_reason {
            let (width, height) =
                ui::Formatted::compute_size(renderer, disconnect_reason, 600.0, 1.0);
            let background = ui::ImageBuilder::new()
                .texture("leafish:solid")
                .position(0.0, 3.0)
                .size(
                    width.max(renderer.ui.size_of_string("Disconnected")) + 4.0,
                    height + 4.0 + 16.0,
                )
                .colour((0, 0, 0, 100))
                .alignment(ui::VAttach::Top, ui::HAttach::Center)
                .draw_index(10)
                .create(ui_container);
            ui::TextBuilder::new()
                .text("Disconnected")
                .position(0.0, 2.0)
                .colour((255, 0, 0, 255))
                .alignment(ui::VAttach::Top, ui::HAttach::Center)
                .attach(&mut *background.borrow_mut());
            ui::FormattedBuilder::new()
                .text(disconnect_reason.clone())
                .position(0.0, 18.0)
                .max_width(600.0)
                .alignment(ui::VAttach::Top, ui::HAttach::Center)
                .attach(&mut *background.borrow_mut());
            Some(background)
        } else {
            None
        };

        let background = if Renderer::get_texture_optional(
            renderer.get_textures_ref(),
            &*format!("#{}", self.background_image),
        )
            .is_some()
        {
            Some(
                ui::ImageBuilder::new()
                    .texture(&*format!("#{}", self.background_image))
                    .size(renderer.safe_width as f64, renderer.safe_height as f64)
                    .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                    .create(ui_container),
            )
        } else {
            None
        };

        self.elements = Some(UIElements {
            logo,
            servers: vec![],

            _add_btn: add,
            _refresh_btn: refresh,
            _options_btn: options,
            _disclaimer: disclaimer,

            _disconnected: disconnected,
            _background: background,
        });
    }*/
}

impl super::Screen for Launcher {

    fn on_active(&mut self, renderer: &mut render::Renderer, ui_container: &mut ui::Container) {
        let background = if Renderer::get_texture_optional(
            renderer.get_textures_ref(),
            &*format!("#{}", self.background_image),
        )
            .is_some()
        {
            Some(
                ui::ImageBuilder::new()
                    .texture(&*format!("#{}", self.background_image))
                    .size(renderer.safe_width as f64, renderer.safe_height as f64)
                    .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                    .create(ui_container),
            )
        } else {
            None
        };
        self.background = background;
        // Options menu
        let options = ui::ButtonBuilder::new()
            .position(5.0, 25.0)
            .size(40.0, 40.0)
            .draw_index(1)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Right)
            .create(ui_container);
        {
            let mut options = options.borrow_mut();
            ui::ImageBuilder::new()
                .texture("leafish:gui/cog")
                .position(0.0, 0.0)
                .size(40.0, 40.0)
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *options);
            options.add_click_func(|_, game| {
                game.screen_sys
                    .clone()
                    .add_screen(Box::new(super::SettingsMenu::new(game.vars.clone(), false)));
                true
            });
        }
        self.options.replace(options);

        // Disclaimer
        let disclaimer = ui::TextBuilder::new()
            .text("Not affiliated with Mojang/Minecraft")
            .position(5.0, 5.0)
            .colour((255, 200, 200, 255))
            .draw_index(1)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Right)
            .create(ui_container);
        self.disclaimer.replace(disclaimer);
        for account in self.accounts.iter() {
            self.rendered_accounts.push(RenderAccount {
                head_picture: None,
                account_name: None,
            });
        }
    }

    fn on_deactive(&mut self, _renderer: &mut render::Renderer, _ui_container: &mut ui::Container) {
        // Clean up
        self.background.take();
        self.options.take();
        self.disclaimer.take();
        self.rendered_accounts.clear();
    }

    fn tick(
        &mut self,
        _: f64,
        _renderer: &mut render::Renderer,
        _ui_container: &mut ui::Container,
    ) -> Option<Box<dyn super::Screen>> {
        // self.logo.tick(renderer);
        None
    }

    fn on_resize(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        self.on_deactive(renderer, ui_container);
        self.on_active(renderer, ui_container);
    }

    /*
    fn on_scroll(&mut self, _: f64, y: f64) {
        if self.displayed_accounts.is_empty() {
            return;
        }
        let mut diff = y / 1.0;
        {
            let last = self.displayed_accounts.last().unwrap();
            if last.offset + diff <= 2.0 {
                diff = 2.0 - last.offset;
            }
            let first = self.displayed_accounts.first().unwrap();
            if first.offset + diff >= 0.0 {
                diff = -first.offset;
            }
        }

        for s in &mut self.displayed_accounts {
            s.offset += diff;
            s.update_position();
        }
    }*/

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}
