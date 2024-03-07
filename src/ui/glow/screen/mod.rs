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

mod server_list;
use crate::Game;

pub use self::server_list::*;
mod login;
pub use self::login::*;

pub mod confirm_box;
pub mod connecting;
pub mod edit_server;

pub mod background;
pub mod chat;
pub mod edit_account;
pub mod launcher;
pub mod respawn;
pub mod settings_menu;

pub use self::settings_menu::SettingsMenu;

use parking_lot::{Mutex, RwLock};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Arc;
use winit::dpi::{PhysicalPosition, Position};
use winit::keyboard::Key;
use winit::keyboard::NamedKey;
use winit::keyboard::PhysicalKey;
use winit::window::Window;

use super::ctx;
use super::render::Renderer;
use super::ui::Container;

pub trait Screen {
    // Called once
    fn init(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
    }
    fn deinit(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
    }

    // May be called multiple times
    fn on_active(
        &mut self,
        screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    );
    fn on_deactive(
        &mut self,
        screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    );

    // Called every frame the screen is active
    fn tick(
        &mut self,
        screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        delta: f64,
    );

    // Events
    fn on_scroll(&mut self, _x: f64, _y: f64) {}

    fn on_resize(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
    } // TODO: make non-optional!

    fn on_key_press(&mut self, key: (Key, PhysicalKey), down: bool, _game: &Arc<Game>) {
        if key.0 == Key::Named(NamedKey::Escape) && !down && self.is_closable() {
            ctx().screen_sys.pop_screen();
        }
    }

    fn is_closable(&self) -> bool {
        false
    }

    fn is_tick_always(&self) -> bool {
        false
    }

    fn ty(&self) -> ScreenType {
        ScreenType::Other(String::new())
    }

    fn clone_screen(&self) -> Box<dyn Screen>;
}

impl Clone for Box<dyn Screen> {
    fn clone(&self) -> Box<dyn Screen> {
        self.clone_screen()
    }
}

#[derive(Eq, PartialEq)]
pub enum ScreenType {
    Other(String),
    Chat,
    InGame,
}

#[derive(Clone)]
struct ScreenInfo {
    screen: Arc<Mutex<Box<dyn Screen>>>,
    active: bool,
    last_width: i32,
    last_height: i32,
}

// TODO: Add safety comment!
unsafe impl Send for ScreenSystem {}
unsafe impl Sync for ScreenSystem {}

#[derive(Default, Clone)]
pub struct ScreenSystem {
    screens: Arc<RwLock<Vec<ScreenInfo>>>,
    pre_computed_screens: Arc<RwLock<Vec<Box<dyn Screen>>>>,
    lowest_offset: Arc<AtomicIsize>,
}

impl ScreenSystem {
    pub fn new() -> ScreenSystem {
        Default::default()
    }

    pub fn add_screen(&self, screen: Box<dyn Screen>) {
        let new_offset = self.pre_computed_screens.clone().read().len() as isize;
        self.pre_computed_screens.clone().write().push(screen);
        let curr_offset = self.lowest_offset.load(Ordering::Acquire);
        if curr_offset == -1 {
            self.lowest_offset.store(new_offset, Ordering::Release);
        }
    }

    pub fn close_closable_screens(&self) {
        while self.is_current_closable() {
            self.pop_screen();
        }
    }

    pub fn pop_screen(&self) {
        if self.pre_computed_screens.clone().read().last().is_some() {
            // TODO: Improve thread safety (becuz of possible race conditions (which are VERY UNLIKELY to happen - and only if screens get added and removed very fast (in one tick)))
            self.pre_computed_screens.clone().write().pop();
            let curr_offset = self.lowest_offset.load(Ordering::Acquire);
            let new_offset = self.pre_computed_screens.clone().read().len() as isize;
            if curr_offset == -1 || new_offset < curr_offset {
                self.lowest_offset.store(new_offset, Ordering::Release);
            }
        }
    }

    pub fn replace_screen(&self, screen: Box<dyn Screen>) {
        self.pop_screen();
        self.add_screen(screen);
    }

    pub fn is_current_closable(&self) -> bool {
        if let Some(last) = self.pre_computed_screens.clone().read().last() {
            return last.is_closable();
        }
        false
    }

    pub fn is_current_ingame(&self) -> bool {
        if let Some(last) = self.pre_computed_screens.clone().read().last() {
            return last.ty() == ScreenType::InGame;
        }
        false
    }

    pub fn is_any_ingame(&self) -> bool {
        for screen in self.pre_computed_screens.clone().read().iter().rev() {
            if screen.ty() == ScreenType::InGame {
                return true;
            }
        }
        false
    }

    pub fn current_screen_ty(&self) -> ScreenType {
        if let Some(last) = self.pre_computed_screens.clone().read().last() {
            return last.ty();
        }
        ScreenType::Other(String::new())
    }

    pub fn press_key(&self, key: (Key, PhysicalKey), down: bool, game: &Arc<Game>) {
        if let Some(screen) = self.screens.clone().read().last() {
            screen.screen.clone().lock().on_key_press(key, down, game);
        }
    }

    #[allow(unused_must_use)]
    pub fn tick(
        self: &Arc<Self>,
        delta: f64,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        window: &Window,
    ) -> bool {
        let lowest = self.lowest_offset.load(Ordering::Acquire);
        if lowest != -1 {
            let screens_len = self.screens.read().len();
            let was_closable = if screens_len > 0 {
                self.screens
                    .read()
                    .last()
                    .as_ref()
                    .unwrap()
                    .screen
                    .lock()
                    .is_closable()
            } else {
                false
            };
            if lowest <= screens_len as isize {
                for _ in 0..(screens_len as isize - lowest) {
                    let screen = self.screens.clone().write().pop().unwrap();
                    if screen.active {
                        screen.screen.clone().lock().on_deactive(
                            self,
                            renderer,
                            ui_container,
                        );
                    }
                    screen
                        .screen
                        .clone()
                        .lock()
                        .deinit(self, &renderer, ui_container);
                }
            }
            for screen in self
                .pre_computed_screens
                .read()
                .iter()
                .skip(lowest as usize)
            {
                let idx = (self.screens.read().len() as isize - 1).max(0) as usize;
                #[allow(clippy::arc_with_non_send_sync)]
                self.screens.write().push(ScreenInfo {
                    screen: Arc::new(Mutex::new(screen.clone())),
                    active: false,
                    last_width: -1,
                    last_height: -1,
                });
                let mut screens = self.screens.write();
                let last = screens.get_mut(idx);
                if let Some(last) = last {
                    if last.active {
                        last.active = false;
                        last.screen.clone().lock().on_deactive(
                            self,
                            renderer,
                            ui_container,
                        );
                    }
                }
                let current = screens.last_mut().unwrap();
                current
                    .screen
                    .clone()
                    .lock()
                    .init(self, renderer, ui_container);
                current.active = true;
                current
                    .screen
                    .clone()
                    .lock()
                    .on_active(self, renderer, ui_container);
            }
            self.lowest_offset.store(-1, Ordering::Release);
            if !was_closable {
                window.set_cursor_position(Position::Physical(PhysicalPosition::new(
                    (renderer.screen_data.read().safe_width / 2) as i32,
                    (renderer.screen_data.read().safe_height / 2) as i32,
                )));
            }
        }

        let len = self.screens.read().len();
        if len == 0 {
            return true;
        }
        // Update state for screens
        {
            let tmp = self.screens.clone();
            let mut tmp = tmp.write();
            let current = tmp.last_mut().unwrap();
            if !current.active {
                current.active = true;
                current
                    .screen
                    .clone()
                    .lock()
                    .on_active(self, renderer, ui_container);
            }
            if current.last_width != renderer.screen_data.read().safe_width as i32
                || current.last_height != renderer.screen_data.read().safe_height as i32
            {
                if current.last_width != -1 && current.last_height != -1 {
                    for screen in tmp.iter_mut().enumerate() {
                        if screen.1.screen.clone().lock().is_tick_always() || screen.0 == len - 1 {
                            screen.1.screen.clone().lock().on_resize(
                                self,
                                &renderer,
                                ui_container,
                            );
                            screen.1.last_width = renderer.screen_data.read().safe_width as i32;
                            screen.1.last_height = renderer.screen_data.read().safe_height as i32;
                        }
                    }
                } else {
                    current.last_width = renderer.screen_data.read().safe_width as i32;
                    current.last_height = renderer.screen_data.read().safe_height as i32;
                }
            }
            for screen in tmp.iter_mut().enumerate() {
                if screen.1.screen.clone().lock().is_tick_always() || screen.0 == len - 1 {
                    screen.1.screen.clone().lock().tick(
                        self,
                        &renderer,
                        ui_container,
                        delta,
                    );
                }
            }
        }
        // Handle current
        return self.screens.read()[len - 1]
            .screen
            .clone()
            .lock()
            .ty()
            != ScreenType::InGame;
    }

    pub fn on_scroll(&self, x: f64, y: f64) {
        if let Some(screen) = self.screens.read().last() {
            screen.screen.lock().on_scroll(x, y);
        }
    }
}
