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

use std::cmp::Ordering;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use log::debug;
use parking_lot::RwLock;
use rand::rngs::ThreadRng;
use rand::Rng;

use crate::inventory::player_inventory::PlayerInventory;
use crate::inventory::{Inventory, Item};
use crate::render;
use crate::render::Renderer;
use crate::screen::Screen;
use crate::ui;
use crate::ui::{Container, HAttach, ImageRef, TextRef, VAttach};

// Textures can be found at: assets/minecraft/textures/gui/icons.png

// TODO: read out "regen: bool"
#[allow(dead_code)]
pub struct HudContext {
    pub enabled: bool,
    pub debug: bool,
    fps: u32,
    dirty_debug: bool,
    hardcore: bool,
    wither: bool,
    poison: bool,
    regen: bool,
    absorbtion: f32,
    last_health_update: u128,
    last_health: f32,
    health: f32,
    max_health: f32,
    dirty_health: bool,
    hunger: bool,
    saturation: u8,
    last_food_update: u128,
    last_food: u8,
    food: u8,
    dirty_food: bool,
    armor: u8,
    dirty_armor: bool,
    exp: f32,
    exp_level: i32,
    dirty_exp: bool,
    breath: i16,
    dirty_breath: bool,
    pub player_inventory: Option<Arc<RwLock<PlayerInventory>>>,
    pub dirty_slots: bool,
    slot_index: u8,
    dirty_slot_index: bool,
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
            player_inventory: None,
            dirty_slots: false,
            slot_index: 0,
            dirty_slot_index: false,
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
}

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
    slot_index_elements: Vec<ImageRef>,
    debug_elements: Vec<TextRef>,
    hud_context: Arc<RwLock<HudContext>>,
    random: ThreadRng,
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
            slot_index_elements: vec![],
            debug_elements: vec![],
            hud_context,
            random: rand::thread_rng(),
        }
    }
}

impl Screen for Hud {
    fn on_active(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        if self.hud_context.clone().read().enabled {
            self.render_health(renderer, ui_container);
            self.render_armor(renderer, ui_container);
            self.render_slots(renderer, ui_container);
            self.render_exp(renderer, ui_container);
            self.render_crosshair(renderer, ui_container);
            self.render_food(renderer, ui_container);
            self.render_breath(renderer, ui_container);
            self.render_slots_items(renderer, ui_container);
            self.render_slot_index(renderer, ui_container);
        }
    }

    fn on_deactive(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {
        self.elements.clear();
        self.health_elements.clear();
        self.exp_elements.clear();
        self.exp_text_elements.clear();
        self.food_elements.clear();
        self.armor_elements.clear();
        self.breath_elements.clear();
        self.slot_elements.clear();
        self.slot_index_elements.clear();
        self.debug_elements.clear();
    }

    fn tick(
        &mut self,
        _delta: f64,
        renderer: &mut render::Renderer,
        ui_container: &mut ui::Container,
    ) -> Option<Box<dyn Screen>> {
        if !self.hud_context.clone().read().enabled {
            if self.last_enabled {
                self.on_deactive(renderer, ui_container);
                self.last_enabled = false;
            }
            return None;
        }
        if self.hud_context.clone().read().enabled && !self.last_enabled {
            self.on_active(renderer, ui_container);
            self.last_enabled = true;
            return None;
        }
        if self.hud_context.clone().read().debug {
            self.render_debug(renderer, ui_container);
            self.last_debug_enabled = true;
        } else if self.last_debug_enabled {
            self.debug_elements.clear();
            self.last_debug_enabled = false;
        }
        if self.hud_context.clone().read().dirty_health {
            self.health_elements.clear();
            self.render_health(renderer, ui_container);
        }
        if self.hud_context.clone().read().dirty_armor {
            self.armor_elements.clear();
            self.render_armor(renderer, ui_container);
        }
        if self.hud_context.clone().read().dirty_food {
            self.food_elements.clear();
            self.render_food(renderer, ui_container);
        }
        if self.hud_context.clone().read().dirty_exp {
            self.exp_elements.clear();
            self.exp_text_elements.clear();
            self.render_exp(renderer, ui_container);
        }
        if self.hud_context.clone().read().dirty_breath {
            self.breath_elements.clear();
            self.render_breath(renderer, ui_container);
        }
        if self.hud_context.clone().read().dirty_slots {
            self.slot_elements.clear();
            self.render_slots_items(renderer, ui_container);
        }
        if self.hud_context.clone().read().dirty_slot_index {
            self.slot_index_elements.clear();
            self.render_slot_index(renderer, ui_container);
        }
        if self.hud_context.clone().read().dirty_debug {
            self.debug_elements.clear();
            self.render_debug(renderer, ui_container);
        }
        None
    }

    fn on_resize(
        &mut self,
        _width: u32,
        _height: u32,
        _renderer: &mut Renderer,
        _ui_container: &mut Container,
    ) {
        if self.hud_context.clone().read().enabled {
            self.on_deactive(_renderer, _ui_container);
            self.on_active(_renderer, _ui_container);
        }
    }

    fn is_closable(&self) -> bool {
        false
    }
}

impl Hud {
    pub fn icon_scale(renderer: &Renderer) -> f64 {
        Hud::icon_scale_by_height(renderer.safe_height)
    }

    pub fn icon_scale_by_height(height: u32) -> f64 {
        let icon_scale = if height > 500 {
            height as f64 / 36.50
        } else {
            height as f64 / 26.50 /*27.5*/
        };
        icon_scale / 9.0
    }

    fn render_health(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
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
            16 + 36
        } else if hud_context.wither {
            16 + 72
        } else {
            16
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
                .texture_coords((
                    (16.0 + updated_offset) as f64 / 256.0,
                    (9.0 * hardcore_offset) as f64 / 256.0,
                    9.0 / 256.0,
                    9.0 / 256.0,
                ))
                .position(x, y)
                .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                .size(icon_scale * 9.0, icon_scale * 9.0)
                .texture("minecraft:gui/icons")
                .create(ui_container);
            self.health_elements.push(image);

            if updated_health {
                if heart as f32 * 2.0 + 1.0 < last_health {
                    let image = ui::ImageBuilder::new()
                        .texture_coords((
                            (texture_offset + 54) as f64 / 256.0,
                            (9.0 * hardcore_offset) as f64 / 256.0,
                            9.0 / 256.0,
                            9.0 / 256.0,
                        ))
                        .position(x, y)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                } else if heart as f32 * 2.0 + 1.0 == last_health {
                    let image = ui::ImageBuilder::new()
                        .texture_coords((
                            (texture_offset + 63) as f64 / 256.0,
                            (9.0 * hardcore_offset) as f64 / 256.0,
                            9.0 / 256.0,
                            9.0 / 256.0,
                        ))
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
                        .texture_coords((
                            (texture_offset + 153) as f64 / 256.0,
                            (9.0 * hardcore_offset) as f64 / 256.0,
                            9.0 / 256.0,
                            9.0 / 256.0,
                        ))
                        .position(x, y)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                } else {
                    let image = ui::ImageBuilder::new()
                        .texture_coords((
                            (texture_offset + 144) as f64 / 256.0,
                            (9.0 * hardcore_offset) as f64 / 256.0,
                            9.0 / 256.0,
                            9.0 / 256.0,
                        ))
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
                        .texture_coords((
                            (texture_offset + 36) as f64 / 256.0,
                            (9.0 * hardcore_offset) as f64 / 256.0,
                            9.0 / 256.0,
                            9.0 / 256.0,
                        ))
                        .position(x, y)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                }

                if heart * 2 + 1 == hp as isize {
                    let image = ui::ImageBuilder::new()
                        .texture_coords((
                            (texture_offset + 45) as f64 / 256.0,
                            (9.0 * hardcore_offset) as f64 / 256.0,
                            9.0 / 256.0,
                            9.0 / 256.0,
                        ))
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

    fn render_armor(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
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
                let y = y_offset + (icon_bars as f64 * (icon_scale * 9.0 + (icon_scale * 1.0)));
                let texture_offset = match (i * 2 + 1).cmp(&armor) {
                    Ordering::Greater => 16.0,
                    Ordering::Equal => 25.0,
                    Ordering::Less => 34.0,
                };
                let image = ui::ImageBuilder::new()
                    .texture_coords((
                        texture_offset / 256.0,
                        9.0 / 256.0,
                        9.0 / 256.0,
                        9.0 / 256.0,
                    ))
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

    fn render_food(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
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
                .texture_coords((
                    (16.0 + j8 * 9.0) / 256.0,
                    27.0 / 256.0,
                    9.0 / 256.0,
                    9.0 / 256.0,
                ))
                .position(x, y_offset)
                .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                .size(icon_scale * 9.0, icon_scale * 9.0)
                .texture("minecraft:gui/icons")
                .create(ui_container);
            self.food_elements.push(image);

            match (i * 2 + 1).cmp(&food) {
                Ordering::Less => {
                    let image = ui::ImageBuilder::new()
                        .texture_coords((
                            (l7 + 36.0) / 256.0,
                            27.0 / 256.0,
                            9.0 / 256.0,
                            9.0 / 256.0,
                        ))
                        .position(x, y_offset)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.food_elements.push(image);
                }
                Ordering::Equal => {
                    let image = ui::ImageBuilder::new()
                        .texture_coords((
                            (l7 + 45.0) / 256.0,
                            27.0 / 256.0,
                            9.0 / 256.0,
                            9.0 / 256.0,
                        ))
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

    fn render_exp(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
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
                .texture_coords((0.0 / 256.0, 64.0 / 256.0, 182.0 / 256.0, 5.0 / 256.0))
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
                    .texture_coords((
                        0.0 / 256.0,
                        69.0 / 256.0,
                        scaled_length as f64 / 256.0,
                        5.0 / 256.0,
                    ))
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

    fn render_slots(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer);
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 256.0, 0.0 / 256.0, 182.0 / 256.0, 22.0 / 256.0))
            .position(0.0, 0.0)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
            .size(icon_scale * 182.0, icon_scale * 22.0)
            .texture("minecraft:gui/widgets")
            .create(ui_container);
        self.elements.push(image);
    }

    // TODO: make use of "render_scoreboard"
    #[allow(dead_code)]
    fn render_scoreboard(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {}

    // TODO: make use of "render_title"
    #[allow(dead_code)]
    fn render_title(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {}

    fn render_slots_items(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer);
        for i in 0..9 {
            if let Some(player_inventory) =
                self.hud_context.clone().read().player_inventory.as_ref()
            {
                let player_inventory = player_inventory.clone();
                let player_inventory = player_inventory.read();
                let item = player_inventory.get_item(36 + i as i16);
                if let Some(item) = item {
                    let slot = self.draw_item(
                        item,
                        -(icon_scale * 90.0) + (i as f64 * (icon_scale * 20.0)) + icon_scale * 11.0,
                        icon_scale * 3.0,
                        ui_container,
                        renderer,
                    );
                    self.slot_elements.push(slot);
                }
            }
        }
        self.hud_context.clone().write().dirty_slots = false;
    }

    fn render_slot_index(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer);
        let slot = self.hud_context.clone().read().slot_index as f64;
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 256.0, 22.0 / 256.0, 24.0 / 256.0, 22.0 / 256.0))
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

    // TODO: make use of "render_item"
    #[allow(dead_code)]
    fn render_item(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {}

    fn render_crosshair(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer);
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 256.0, 0.0 / 256.0, 16.0 / 256.0, 16.0 / 256.0))
            .position(0.0, 0.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .size(icon_scale * 16.0, icon_scale * 16.0)
            .texture("minecraft:gui/icons")
            .create(ui_container);
        self.elements.push(image);
    }

    fn render_breath(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
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
                        .texture_coords((16.0 / 256.0, 18.0 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                        .position(x, y_offset)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale * 9.0, icon_scale * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.elements.push(image);
                } else {
                    // broken bubble
                    let image = ui::ImageBuilder::new()
                        .texture_coords((25.0 / 256.0, 18.0 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
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

    pub fn render_debug(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let hud_context = self.hud_context.clone();
        let hud_context = hud_context.read();
        let icon_scale = Hud::icon_scale(renderer);
        let scale = icon_scale / 2.0;
        self.debug_elements.push(
            ui::TextBuilder::new()
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

    pub fn draw_item(
        &self,
        item: &Item,
        x: f64,
        y: f64,
        ui_container: &mut Container,
        renderer: &Renderer,
    ) -> ImageRef {
        let icon_scale = Hud::icon_scale(renderer);
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 16.0, 0.0 / 16.0, 1.0, 1.0))
            .position(x, y)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
            .size(icon_scale * 16.0, icon_scale * 16.0)
            .texture(format!("minecraft:{}", item.material.texture_location()))
            .create(ui_container);
        image
    }
}
