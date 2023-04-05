// Copyright 2021 Terrarier2111
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

use std::cmp;
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use log::debug;
use parking_lot::RwLock;
use rand::rngs::ThreadRng;
use rand::Rng;

use crate::inventory::slot_mapping::SlotMapping;
use crate::inventory::Item;
use crate::render;
use crate::render::Renderer;
use crate::screen::{Screen, ScreenSystem, ScreenType};
use crate::server::Server;
use crate::ui;
use crate::ui::{Container, FormattedRef, HAttach, ImageRef, TextRef, VAttach};
use crate::{format, screen, settings, Game};
use leafish_protocol::protocol::packet::play::serverbound::HeldItemChange;
use leafish_protocol::types::GameMode;
use std::sync::atomic::AtomicBool;

use glutin::event::VirtualKeyCode;
use instant::Instant;
use std::sync::atomic::Ordering as AtomicOrdering;

// Textures can be found at: assets/minecraft/textures/gui/icons.png

// TODO: read out "regen: bool"
#[allow(dead_code)]
pub struct HudContext {
    pub enabled: bool,
    pub debug: bool,
    fps: u32,
    dirty_debug: bool,
    hardcore: bool,  // TODO: Update this!
    wither: bool,    // TODO: Update this!
    poison: bool,    // TODO: Update this!
    regen: bool,     // TODO: Update this!
    absorbtion: f32, // TODO: Update this!
    last_health_update: u128,
    last_health: f32,
    health: f32,
    max_health: f32, // TODO: Update this!
    dirty_health: bool,
    hunger: bool, // TODO: Update this!
    saturation: u8,
    last_food_update: u128,
    last_food: u8,
    food: u8,
    dirty_food: bool,
    armor: u8, // TODO: Update this!
    dirty_armor: bool,
    pub exp: f32,
    pub exp_level: i32,
    dirty_exp: bool,
    breath: i16, // TODO: Update this!
    dirty_breath: bool,
    pub slots: Option<Arc<RwLock<SlotMapping>>>,
    pub server: Option<Arc<Server>>,
    pub dirty_slots: AtomicBool,
    slot_index: u8,
    dirty_slot_index: bool,
    pub game_mode: GameMode,
    dirty_game_mode: bool,
}

impl Default for render::hud::HudContext {
    fn default() -> Self {
        Self::new()
    }
}
impl HudContext {
    pub fn new() -> Self {
        HudContext {
            enabled: true,
            debug: false,
            fps: 0,
            dirty_debug: false,
            hardcore: false,
            wither: false,
            poison: false,
            regen: false,
            absorbtion: 0.0,
            last_health_update: 0,
            last_health: 0.0,
            health: 20.0,
            max_health: 20.0,
            dirty_health: false,
            hunger: false,
            saturation: 0,
            last_food_update: 0,
            last_food: 0,
            food: 20,
            dirty_food: false,
            armor: 0,
            dirty_armor: false,
            exp: 0.0, // 0.0 - 1.0
            exp_level: 0,
            dirty_exp: false,
            breath: 0, /*-1*/
            // -1 = disabled (not under water) | 1 bubble = 30 | +2 = broken bubble -- -1 is causing crashes when attempting to join servers!
            dirty_breath: false,
            slots: None,
            server: None,
            dirty_slots: AtomicBool::new(false),
            slot_index: 0,
            dirty_slot_index: false,
            game_mode: GameMode::Survival,
            dirty_game_mode: false,
        }
    }

    // TODO: Implement effects!

    pub fn update_health_and_food(&mut self, health: f32, food: u8, saturation: u8) {
        let start = SystemTime::now();
        let time = start.duration_since(UNIX_EPOCH).unwrap().as_millis();
        self.last_health_update = time;
        self.last_health = self.health;
        self.health = health;
        self.last_food_update = time;
        self.last_food = self.food;
        self.food = food;
        self.saturation = saturation;
        self.dirty_food = true;
        self.dirty_health = true;
        self.dirty_armor = true; // We have to redraw the armor too, because it depends on the number of hearts and absorbtion.
    }

    pub fn update_max_health(&mut self, max_health: f32) {
        self.max_health = max_health;
        self.dirty_health = true;
        self.dirty_armor = true; // We have to redraw the armor too, because it depends on the number of hearts and absorbtion.
    }

    pub fn update_absorbtion(&mut self, absorbtion: f32) {
        self.absorbtion = absorbtion;
        self.dirty_health = true;
        self.dirty_armor = true; // We have to redraw the armor too, because it depends on the number of hearts and absorbtion.
    }

    pub fn update_armor(&mut self, armor: u8) {
        self.armor = armor;
        self.dirty_armor = true;
    }

    pub fn update_breath(&mut self, breath: i16) {
        self.breath = breath;
        self.dirty_breath = true;
    }

    pub fn update_exp(&mut self, exp: f32, level: i32) {
        self.exp = exp;
        self.exp_level = level;
        self.dirty_exp = true;
    }

    pub fn update_slot_index(&mut self, slot_index: u8) {
        self.slot_index = slot_index;
        self.dirty_slot_index = true;
    }

    pub fn update_fps(&mut self, fps: u32) {
        self.fps = fps;
        if self.debug {
            self.dirty_debug = true;
        }
    }

    pub fn update_game_mode(&mut self, game_mode: GameMode) {
        self.game_mode = game_mode;
        self.dirty_game_mode = true;
    }

    pub fn get_slot_index(&self) -> u8 {
        self.slot_index
    }

    pub fn display_message_in_chat(&mut self, message: format::Component) {
        self.server
            .as_ref()
            .unwrap()
            .chat_ctx
            .clone()
            .push_msg(message);
    }
}

#[derive(Clone)]
pub struct Hud {
    last_enabled: bool,
    last_debug_enabled: bool,
    elements: Vec<ImageRef>,
    health_elements: Vec<ImageRef>,
    armor_elements: Vec<ImageRef>,
    food_elements: Vec<ImageRef>,
    breath_elements: Vec<ImageRef>,
    exp_elements: Vec<ImageRef>,
    exp_text_elements: Vec<TextRef>,
    slot_elements: Vec<ImageRef>,
    slot_text_elements: Vec<TextRef>,
    slot_index_elements: Vec<ImageRef>,
    debug_elements: Vec<TextRef>,
    chat_elements: Vec<FormattedRef>,
    chat_background_elements: Vec<ImageRef>,
    hud_context: Arc<RwLock<HudContext>>,
    random: ThreadRng,
    last_tick: Instant,
    render_chat: bool,
}

impl Hud {
    pub fn new(hud_context: Arc<RwLock<HudContext>>) -> Self {
        Hud {
            last_enabled: true,
            last_debug_enabled: false,
            elements: vec![],
            health_elements: vec![],
            armor_elements: vec![],
            food_elements: vec![],
            breath_elements: vec![],
            exp_elements: vec![],
            exp_text_elements: vec![],
            slot_elements: vec![],
            slot_text_elements: vec![],
            slot_index_elements: vec![],
            debug_elements: vec![],
            chat_elements: vec![],
            chat_background_elements: vec![],
            hud_context,
            random: rand::thread_rng(),
            last_tick: Instant::now(),
            render_chat: false,
        }
    }
}

impl Screen for Hud {
    fn init(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        if self.hud_context.clone().read().enabled {
            self.render_slots(renderer.clone(), ui_container);
            self.render_slots_items(renderer.clone(), ui_container);
            self.render_slot_index(renderer.clone(), ui_container);
            self.render_crosshair(renderer.clone(), ui_container);
            self.render_chat(renderer.clone(), ui_container);
            let game_mode = self.hud_context.clone().read().game_mode;
            if matches!(game_mode, GameMode::Adventure | GameMode::Survival) {
                self.render_health(renderer.clone(), ui_container);
                self.render_armor(renderer.clone(), ui_container);
                self.render_exp(renderer.clone(), ui_container);
                self.render_food(renderer.clone(), ui_container);
                self.render_breath(renderer, ui_container);
            }
        }
    }

    fn deinit(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
        self.elements.clear();
        self.health_elements.clear();
        self.exp_elements.clear();
        self.exp_text_elements.clear();
        self.food_elements.clear();
        self.armor_elements.clear();
        self.breath_elements.clear();
        self.slot_elements.clear();
        self.slot_text_elements.clear();
        self.slot_index_elements.clear();
        self.debug_elements.clear();
        self.chat_elements.clear();
        self.chat_background_elements.clear();
    }

    fn on_active(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
    }

    fn on_deactive(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
    }

    fn tick(
        &mut self,
        screen_sys: &ScreenSystem,
        renderer: Arc<render::Renderer>,
        ui_container: &mut ui::Container,
        _delta: f64,
    ) {
        if !self.hud_context.clone().read().enabled {
            if self.last_enabled {
                self.on_deactive(screen_sys, renderer, ui_container);
                self.last_enabled = false;
            }
            return;
        }
        if self.hud_context.clone().read().enabled && !self.last_enabled {
            self.on_active(screen_sys, renderer, ui_container);
            self.last_enabled = true;
            return;
        }
        if self.hud_context.clone().read().debug {
            self.render_debug(renderer.clone(), ui_container);
            self.last_debug_enabled = true;
        } else if self.last_debug_enabled {
            self.debug_elements.clear();
            self.last_debug_enabled = false;
        }
        let game_mode = self.hud_context.clone().read().game_mode;
        if self.hud_context.clone().read().dirty_game_mode {
            self.hud_context.clone().write().dirty_game_mode = false;
            if matches!(game_mode, GameMode::Adventure | GameMode::Survival) {
                if self.health_elements.is_empty() {
                    self.render_health(renderer.clone(), ui_container);
                    self.render_armor(renderer.clone(), ui_container);
                    self.render_exp(renderer.clone(), ui_container);
                    self.render_food(renderer.clone(), ui_container);
                    self.render_breath(renderer.clone(), ui_container);
                }
            } else {
                self.health_elements.clear();
                self.armor_elements.clear();
                self.food_elements.clear();
                self.exp_elements.clear();
                self.exp_text_elements.clear();
                self.breath_elements.clear();
            }
        }
        if matches!(game_mode, GameMode::Adventure | GameMode::Survival) {
            if self.hud_context.clone().read().dirty_health {
                self.health_elements.clear();
                self.render_health(renderer.clone(), ui_container);
            }
            if self.hud_context.clone().read().dirty_armor {
                self.armor_elements.clear();
                self.render_armor(renderer.clone(), ui_container);
            }
            if self.hud_context.clone().read().dirty_food {
                self.food_elements.clear();
                self.render_food(renderer.clone(), ui_container);
            }
            if self.hud_context.clone().read().dirty_exp {
                self.exp_elements.clear();
                self.exp_text_elements.clear();
                self.render_exp(renderer.clone(), ui_container);
            }
            if self.hud_context.clone().read().dirty_breath {
                self.breath_elements.clear();
                self.render_breath(renderer.clone(), ui_container);
            }
        }
        if self
            .hud_context
            .clone()
            .read()
            .dirty_slots
            .load(AtomicOrdering::Relaxed)
        {
            self.slot_elements.clear();
            self.slot_text_elements.clear();
            self.render_slots_items(renderer.clone(), ui_container);
        }
        if self.hud_context.clone().read().dirty_slot_index {
            self.slot_index_elements.clear();
            self.render_slot_index(renderer.clone(), ui_container);
        }
        if self.hud_context.clone().read().dirty_debug {
            self.debug_elements.clear();
            self.render_debug(renderer.clone(), ui_container);
        }
        if !self.chat_background_elements.is_empty()
            && screen_sys.current_screen_ty() == ScreenType::Chat
        {
            self.chat_background_elements.clear();
            self.chat_elements.clear();
            self.render_chat = true;
        }
        if (self
            .hud_context
            .clone()
            .read()
            .server
            .as_ref()
            .unwrap()
            .clone()
            .chat_ctx
            .clone()
            .is_dirty()
            || self.render_chat)
            && screen_sys.current_screen_ty() != ScreenType::Chat
        {
            self.render_chat(renderer, ui_container);
            self.render_chat = false;
        }
    }

    fn on_scroll(&mut self, _: f64, y: f64) {
        // TODO: Is there a threshold we have to implement?
        let curr_slot = self
            .hud_context
            .clone()
            .read()
            .server
            .as_ref()
            .unwrap()
            .clone()
            .inventory_context
            .clone()
            .read()
            .hotbar_index;
        let new_slot = if y == -1.0 {
            if curr_slot == 8 {
                0
            } else {
                curr_slot + 1
            }
        } else if y == 1.0 {
            if curr_slot == 0 {
                8
            } else {
                curr_slot - 1
            }
        } else {
            curr_slot
        };
        self.hud_context
            .clone()
            .read()
            .server
            .as_ref()
            .unwrap()
            .clone()
            .write_packet(HeldItemChange {
                slot: new_slot as i16,
            });
        self.hud_context
            .clone()
            .read()
            .server
            .as_ref()
            .unwrap()
            .clone()
            .inventory_context
            .clone()
            .write()
            .hotbar_index = new_slot;
        self.hud_context.clone().write().slot_index = new_slot;
        self.hud_context.clone().write().dirty_slot_index = true;
    }

    fn on_resize(
        &mut self,
        screen_sys: &ScreenSystem,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        if self.hud_context.clone().read().enabled {
            self.deinit(screen_sys, renderer.clone(), ui_container);
            self.init(screen_sys, renderer, ui_container);
        }
    }

    fn on_key_press(&mut self, key: VirtualKeyCode, down: bool, game: &mut Game) {
        if key == VirtualKeyCode::Escape && !down && game.focused {
            game.screen_sys
                .add_screen(Box::new(screen::SettingsMenu::new(game.vars.clone(), true)));
            return;
        }
        if let Some(action_key) = settings::Actionkey::get_by_keycode(key, &game.vars) {
            game.server
                .as_ref()
                .unwrap()
                .key_press(down, action_key, &mut game.focused.clone());
        }
    }

    fn is_closable(&self) -> bool {
        false
    }

    fn is_tick_always(&self) -> bool {
        true
    }

    fn ty(&self) -> ScreenType {
        ScreenType::InGame
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}

impl Hud {
    pub fn icon_scale(renderer: Arc<Renderer>) -> f64 {
        let screen = renderer.screen_data.read();
        Hud::icon_scale_by_dims(screen.safe_width, screen.safe_height)
    }

    pub fn icon_scale_by_dims(width: u32, height: u32) -> f64 {
        // See https://minecraft.fandom.com/wiki/Options#Video_Settings
        (width / 320).min(height / 240).max(1) as f64
    }

    fn render_health(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let hud_context = self.hud_context.clone();
        let hud_context = hud_context.read();
        let icon_scale = Hud::icon_scale(renderer);
        let x_offset = icon_scale * 182.0 / 2.0 * -1.0 + icon_scale * 9.0 / 2.0;
        let y_offset = icon_scale * 30.0;
        let hp = hud_context.health.ceil();
        let max_health = hud_context.max_health;
        let absorbtion = hud_context.absorbtion;
        let last_health = hud_context.last_health;
        let mut tmp_absorbtion = absorbtion;
        let regen_animation = -1; // TODO: Implement regen animation!
        let updated_health = false; // whether health updated recently or not
                                    // TODO: Implement updated health animation!
        let updated_offset = if updated_health { 9.0 } else { 0.0 };
        let hardcore_offset = if hud_context.hardcore { 5.0 } else { 0.0 };
        let texture_offset = if hud_context.poison {
            16.0 + 36.0
        } else if hud_context.wither {
            16.0 + 72.0
        } else {
            16.0
        };
        drop(hud_context);
        let mut redirty_health = false;

        for heart in (0..((((max_health + absorbtion) / 2.0) as f64).ceil()) as isize).rev() {
            let heart_rows = (((heart + 1) as f32 / 10.0).ceil() as f64) - 1.0;
            let x = x_offset + (heart as f64) % 10.0 * (icon_scale * 8.0);
            let mut y = y_offset + (heart_rows * (icon_scale * 9.0 + (icon_scale * 1.0)));

            if heart == regen_animation {
                // This moves the hearts down when the regeneration effect is active
                y -= icon_scale * 2.0;
            }

            if hp <= 4.0 {
                // Creates the jittery effect when player has less than 2.5 hearts
                y += icon_scale * (self.random.gen_range(0..2) as f64);
                redirty_health = true;
            }

            let image = ui::ImageBuilder::new()
                .draw_index(HUD_PRIORITY)
                .texture_coords((16.0 + updated_offset, 9.0 * hardcore_offset, 9.0, 9.0))
                .position(x, y)
                .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                .size(icon_scale * 9.0, icon_scale * 9.0)
                .texture("minecraft:gui/icons")
                .create(ui_container);
            self.health_elements.push(image);

            if updated_health {
                if heart as f32 * 2.0 + 1.0 < last_health {
                    let image = ui::ImageBuilder::new()
                        .draw_index(HUD_PRIORITY)
                        .texture_coords((texture_offset + 54.0, 9.0 * hardcore_offset, 9.0, 9.0))
                        .position(x, y)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                } else if heart as f32 * 2.0 + 1.0 == last_health {
                    let image = ui::ImageBuilder::new()
                        .draw_index(HUD_PRIORITY)
                        .texture_coords((texture_offset + 63.0, 9.0 * hardcore_offset, 9.0, 9.0))
                        .position(x, y)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                }
            }

            if tmp_absorbtion > 0.0 {
                if tmp_absorbtion == absorbtion && absorbtion % 2.0 == 1.0 {
                    let image = ui::ImageBuilder::new()
                        .draw_index(HUD_PRIORITY)
                        .texture_coords((texture_offset + 153.0, 9.0 * hardcore_offset, 9.0, 9.0))
                        .position(x, y)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                } else {
                    let image = ui::ImageBuilder::new()
                        .draw_index(HUD_PRIORITY)
                        .texture_coords((texture_offset + 144.0, 9.0 * hardcore_offset, 9.0, 9.0))
                        .position(x, y)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                }

                tmp_absorbtion -= 2.0;
            } else {
                if heart * 2 + 1 < hp as isize {
                    let image = ui::ImageBuilder::new()
                        .draw_index(HUD_PRIORITY)
                        .texture_coords((texture_offset + 36.0, 9.0 * hardcore_offset, 9.0, 9.0))
                        .position(x, y)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                }

                if heart * 2 + 1 == hp as isize {
                    let image = ui::ImageBuilder::new()
                        .draw_index(HUD_PRIORITY)
                        .texture_coords((texture_offset + 45.0, 9.0 * hardcore_offset, 9.0, 9.0))
                        .position(x, y)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                }
            }
        }
        if !redirty_health {
            self.hud_context.write().dirty_health = false;
        }
    }

    fn render_armor(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let armor = self.hud_context.clone().read().armor;
        let icon_scale = Hud::icon_scale(renderer);
        let x_offset = icon_scale * 182.0 / 2.0 * -1.0 + icon_scale * 9.0 / 2.0;
        let y_offset = icon_scale * 30.0;
        let max_health = self.hud_context.clone().read().max_health;
        let absorbtion = self.hud_context.clone().read().absorbtion;
        let icon_bars = (((max_health + absorbtion) / 2.0 / 10.0) as f64).ceil();

        if armor > 0 {
            for i in 0..10 {
                let x = x_offset + i as f64 * (icon_scale * 8.0);
                let y = y_offset + (icon_bars * (icon_scale * 9.0 + (icon_scale * 1.0)));
                let texture_offset = match (i * 2 + 1).cmp(&armor) {
                    Ordering::Greater => 16.0,
                    Ordering::Equal => 25.0,
                    Ordering::Less => 34.0,
                };
                let image = ui::ImageBuilder::new()
                    .draw_index(HUD_PRIORITY)
                    .texture_coords((texture_offset, 9.0, 9.0, 9.0))
                    .position(x, y)
                    .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                    .size(icon_scale * 9.0, icon_scale * 9.0)
                    .texture("minecraft:gui/icons")
                    .create(ui_container);
                self.armor_elements.push(image);
            }
        }
        self.hud_context.write().dirty_armor = false;
    }

    fn render_food(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer);
        let hud_context = self.hud_context.clone();
        let hud_context = hud_context.read();
        let food = hud_context.food;
        let _last_food = hud_context.last_food;
        let x_offset = icon_scale * 182.0 / 2.0 + icon_scale * 9.0 / 2.0;
        let y_offset = icon_scale * 30.0;

        let mut l7 = 16.0;
        let mut j8 = 0.0;

        if hud_context.hunger {
            l7 += 36.0;
            j8 = 13.0;
        }

        drop(hud_context);

        for i in 0..10 {
            let x = x_offset - i as f64 * (icon_scale * 8.0) - icon_scale * 9.0;
            let image = ui::ImageBuilder::new()
                .draw_index(HUD_PRIORITY)
                .texture_coords(((16.0 + j8 * 9.0), 27.0, 9.0, 9.0))
                .position(x, y_offset)
                .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                .size(icon_scale * 9.0, icon_scale * 9.0)
                .texture("minecraft:gui/icons")
                .create(ui_container);
            self.food_elements.push(image);

            match (i * 2 + 1).cmp(&food) {
                Ordering::Less => {
                    let image = ui::ImageBuilder::new()
                        .draw_index(HUD_PRIORITY)
                        .texture_coords(((l7 + 36.0), 27.0, 9.0, 9.0))
                        .position(x, y_offset)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.food_elements.push(image);
                }
                Ordering::Equal => {
                    let image = ui::ImageBuilder::new()
                        .draw_index(HUD_PRIORITY)
                        .texture_coords(((l7 + 45.0), 27.0, 9.0, 9.0))
                        .position(x, y_offset)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.food_elements.push(image);
                }
                Ordering::Greater => debug!("Nothing happens here, but that's probably wrong"),
            }
        }
        self.hud_context.write().dirty_food = false;
    }

    fn render_exp(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer);
        let y_offset = icon_scale * 24.0;
        let hud_context = self.hud_context.clone();
        let hud_context = hud_context.read();
        let max_exp = if hud_context.exp_level >= 30 {
            112 + (hud_context.exp_level - 30) * 9
        } else if hud_context.exp_level >= 15 {
            37 + (hud_context.exp_level - 15) * 5
        } else {
            7 + hud_context.exp_level * 2
        };
        if max_exp > 0 {
            let image = ui::ImageBuilder::new()
                .draw_index(HUD_PRIORITY)
                .texture_coords((0.0, 64.0, 182.0, 5.0))
                .position(0.0, y_offset)
                .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                .size(icon_scale * 182.0, icon_scale * 5.0)
                .texture("minecraft:gui/icons")
                .create(ui_container);
            self.exp_elements.push(image);

            let scaled_length = hud_context.exp * 182.0;
            if scaled_length > 0.0 {
                let shift = icon_scale * (((182.0) - scaled_length as f64) / 2.0);
                let image = ui::ImageBuilder::new()
                    .draw_index(HUD_PRIORITY)
                    .texture_coords((0.0, 69.0, scaled_length as f64, 5.0))
                    .position(shift * -1.0, y_offset)
                    .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                    .size(icon_scale * scaled_length as f64, icon_scale * 5.0)
                    .texture("minecraft:gui/icons")
                    .create(ui_container);
                self.exp_elements.push(image);
            }
        }
        if hud_context.exp_level > 0 {
            let level_str = format!("{}", hud_context.exp_level);
            let scale = icon_scale / 2.0;
            let y = icon_scale * 26.0;
            self.exp_text_elements.push(
                ui::TextBuilder::new()
                    .draw_index(HUD_PRIORITY)
                    .alignment(VAttach::Bottom, HAttach::Center)
                    .scale_x(scale)
                    .scale_y(scale)
                    .position(icon_scale * 1.0, y)
                    .text(&level_str)
                    .colour((0, 0, 0, 255))
                    .shadow(false)
                    .create(ui_container),
            );
            self.exp_text_elements.push(
                ui::TextBuilder::new()
                    .draw_index(HUD_PRIORITY)
                    .alignment(VAttach::Bottom, HAttach::Center)
                    .scale_x(scale)
                    .scale_y(scale)
                    .position(-(icon_scale * 1.0), y)
                    .text(&level_str)
                    .colour((0, 0, 0, 1))
                    .shadow(false)
                    .create(ui_container),
            );
            self.exp_text_elements.push(
                ui::TextBuilder::new()
                    .draw_index(HUD_PRIORITY)
                    .alignment(VAttach::Bottom, HAttach::Center)
                    .scale_x(scale)
                    .scale_y(scale)
                    .position(0.0, y + (icon_scale * 1.0))
                    .text(&level_str)
                    .colour((0, 0, 0, 255))
                    .shadow(false)
                    .create(ui_container),
            );
            self.exp_text_elements.push(
                ui::TextBuilder::new()
                    .draw_index(HUD_PRIORITY)
                    .alignment(VAttach::Bottom, HAttach::Center)
                    .scale_x(scale)
                    .scale_y(scale)
                    .position(0.0, y - (icon_scale * 1.0))
                    .text(&level_str)
                    .colour((0, 0, 0, 255))
                    .shadow(false)
                    .create(ui_container),
            );
            self.exp_text_elements.push(
                ui::TextBuilder::new()
                    .draw_index(HUD_PRIORITY)
                    .alignment(VAttach::Bottom, HAttach::Center)
                    .scale_x(scale)
                    .scale_y(scale)
                    .position(0.0, y)
                    .text(&level_str)
                    .colour((128, 255, 32, 255))
                    .shadow(false)
                    .create(ui_container),
            );
        }
        drop(hud_context);
        self.hud_context.write().dirty_exp = false;
    }

    fn render_slots(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer);
        let image = ui::ImageBuilder::new()
            .draw_index(HUD_PRIORITY)
            .texture_coords((0.0, 0.0, 182.0, 22.0))
            .position(0.0, 0.0)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
            .size(icon_scale * 182.0, icon_scale * 22.0)
            .texture("minecraft:gui/widgets")
            .create(ui_container);
        self.elements.push(image);
    }

    // TODO: make use of "render_scoreboard"
    #[allow(dead_code)]
    fn render_scoreboard(&mut self, _renderer: Arc<Renderer>, _ui_container: &mut Container) {}

    // TODO: make use of "render_title"
    #[allow(dead_code)]
    fn render_title(&mut self, _renderer: Arc<Renderer>, _ui_container: &mut Container) {}

    fn render_slots_items(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer.clone());
        for i in 0..9 {
            if let Some(inventory) = &self.hud_context.clone().read().slots {
                if let Some(item) = inventory.clone().read().get_item(27 + i as u16) {
                    let (slot_item, stack_count) = self.draw_item(
                        &item,
                        (icon_scale) * -1.0
                            + -(icon_scale * 90.0)
                            + (i as f64 * (icon_scale * 20.0))
                            + icon_scale * 11.0,
                        icon_scale * 3.0,
                        ui_container,
                        renderer.clone(),
                    );
                    self.slot_elements.push(slot_item);
                    if let Some(stack_count) = stack_count {
                        self.slot_text_elements.push(stack_count);
                    }
                }
            }
        }
        self.hud_context
            .clone()
            .write()
            .dirty_slots
            .store(false, AtomicOrdering::Relaxed);
    }

    fn render_slot_index(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer);
        let slot = self.hud_context.clone().read().slot_index as f64;
        let image = ui::ImageBuilder::new()
            .draw_index(HUD_PRIORITY)
            .texture_coords((0.0, 22.0, 24.0, 22.0))
            .position(
                (icon_scale) * -1.0
                    + -(icon_scale * 90.0)
                    + (slot * (icon_scale * 20.0))
                    + icon_scale * 11.0,
                (icon_scale) * 1.0,
            )
            .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
            .size(icon_scale * 24.0, icon_scale * 22.0)
            .texture("minecraft:gui/widgets")
            .create(ui_container);
        self.slot_index_elements.push(image);
        self.hud_context.clone().write().dirty_slot_index = false;
    }

    // TODO: make use of "render_item" (in right hand)
    #[allow(dead_code)]
    fn render_item(&mut self, _renderer: Arc<Renderer>, _ui_container: &mut Container) {}

    fn render_crosshair(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer);
        let image = ui::ImageBuilder::new()
            .draw_index(HUD_PRIORITY)
            .texture_coords((0.0, 0.0, 16.0, 16.0))
            .position(0.0, 0.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .size(icon_scale * 16.0, icon_scale * 16.0)
            .texture("minecraft:gui/icons")
            .create(ui_container);
        self.elements.push(image);
    }

    fn render_breath(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let hud_context = self.hud_context.clone();
        let hud_context = hud_context.read();

        if hud_context.breath != -1 {
            // Whether the player is under water or not.
            let breath = hud_context.breath as f64;
            drop(hud_context);
            let bubbles = ((breath - 2.0) * 10.0 / 300.0).ceil();
            let broken_bubbles = (breath * 10.0 / 300.0).ceil() - bubbles;

            let icon_scale = Hud::icon_scale(renderer);
            let y_offset = icon_scale * 40.0;
            let x_offset = icon_scale * 182.0 / 2.0 + icon_scale * 9.0 / 2.0;

            for i in 0..bubbles as i32 + broken_bubbles as i32 {
                let x = x_offset - i as f64 * (icon_scale / 9.0 * 8.0) - icon_scale;
                if i < (bubbles as i32) {
                    // normal bubble
                    let image = ui::ImageBuilder::new()
                        .draw_index(HUD_PRIORITY)
                        .texture_coords((16.0, 18.0, 9.0, 9.0))
                        .position(x, y_offset)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.elements.push(image);
                } else {
                    // broken bubble
                    let image = ui::ImageBuilder::new()
                        .draw_index(HUD_PRIORITY)
                        .texture_coords((25.0, 18.0, 9.0, 9.0))
                        .position(x, y_offset)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.elements.push(image);
                }
            }
        }
        self.hud_context.write().dirty_breath = false;
    }

    pub fn render_debug(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let hud_context = self.hud_context.clone();
        let hud_context = hud_context.read();
        let icon_scale = Hud::icon_scale(renderer);
        let scale = icon_scale / 2.0;
        self.debug_elements.push(
            ui::TextBuilder::new()
                .draw_index(HUD_PRIORITY)
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(scale)
                .scale_y(scale)
                .position(icon_scale, icon_scale)
                .text(format!("FPS: {}", hud_context.fps))
                .colour((0, 102, 204, 255))
                .shadow(false)
                .create(ui_container),
        );
    }

    pub fn render_chat(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let now = Instant::now();
        if now.duration_since(self.last_tick).as_millis() >= 50 {
            self.last_tick = now;
            self.chat_elements.clear();
            self.chat_background_elements.clear();
            let scale = Hud::icon_scale(renderer.clone());
            let messages = self
                .hud_context
                .clone()
                .read()
                .server
                .as_ref()
                .unwrap()
                .chat_ctx
                .clone()
                .tick_visible_messages();
            let history_size = messages.len();

            let mut component_lines = 0;
            for message in messages.iter().take(cmp::min(10, history_size)) {
                let lines = (renderer.ui.lock().size_of_string(&message.1.to_string())
                    / (CHAT_WIDTH * scale))
                    .ceil() as u8;
                component_lines += lines;
            }

            if history_size > 0 {
                self.chat_background_elements.push(
                    ui::ImageBuilder::new()
                        .draw_index(HUD_PRIORITY + 1)
                        .texture("leafish:solid")
                        .alignment(VAttach::Bottom, HAttach::Left)
                        .position(1.0 * scale, scale * 85.0 / 2.0)
                        .size(
                            500.0 / 2.0 * scale,
                            5.0 * scale * (component_lines as f64)
                                + cmp::min(10, history_size) as f64 * 0.4 * scale,
                        )
                        .colour((0, 0, 0, 100))
                        .create(ui_container),
                );
            }

            let mut component_lines = 0;
            for message in messages.iter().take(cmp::min(10, history_size)).enumerate() {
                let lines = (renderer.ui.lock().size_of_string(&message.1 .1.to_string())
                    / (CHAT_WIDTH * scale))
                    .ceil() as u8;
                let transparency = if message.1 .0 >= FADE_OUT_START_TICKS {
                    1.0
                } else {
                    message.1 .0 as f64 / FADE_OUT_START_TICKS as f64
                };
                let text = ui::FormattedBuilder::new()
                    .draw_index(HUD_PRIORITY + 1)
                    .alignment(VAttach::Bottom, HAttach::Left)
                    .position(
                        1.0 * scale,
                        scale * 85.0 / 2.0
                            + ((component_lines as f64) * 5.0) * scale
                            + message.0 as f64 * 0.4 * scale,
                    )
                    .text(message.1 .1.clone())
                    .transparency(transparency)
                    .max_width(CHAT_WIDTH * scale)
                    .create(ui_container);
                self.chat_elements.push(text);
                component_lines += lines;
            }
        }
    }

    pub fn draw_item(
        &self,
        item: &Item,
        x: f64,
        y: f64,
        ui_container: &mut Container,
        renderer: Arc<Renderer>,
    ) -> (ImageRef, Option<TextRef>) {
        let icon_scale = Hud::icon_scale(renderer.clone());
        let textures = item.material.texture_locations();
        let texture =
            if let Some(tex) = Renderer::get_texture_optional(&renderer.textures, &textures.0) {
                if tex.dummy {
                    textures.1
                } else {
                    textures.0
                }
            } else {
                textures.1
            };
        let item_image = ui::ImageBuilder::new()
            .draw_index(HUD_PRIORITY)
            .texture_coords((0.0, 0.0, 256.0, 256.0))
            .position(x, y)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
            .size(icon_scale * 16.0, icon_scale * 16.0)
            .texture(format!("minecraft:{}", texture))
            .create(ui_container);

        let text = if item.stack.count != 1 {
            Some(
                ui::TextBuilder::new()
                    .scale_x(icon_scale / 2.0)
                    .scale_y(icon_scale / 2.0)
                    .text(item.stack.count.to_string())
                    .position(x - icon_scale * 2.0, y + icon_scale * 5.0)
                    .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                    .colour((255, 255, 255, 255))
                    .shadow(true)
                    .create(ui_container),
            )
        } else {
            None
        };
        (item_image, text)
    }
}

pub const CHAT_WIDTH: f64 = 490.0 / 2.0;
const HUD_PRIORITY: isize = -2;
pub const START_TICKS: usize = 10 * 20;
pub const FADE_OUT_START_TICKS: usize = 20;
