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

use crate::gl;
use crate::render;
use crate::render::{glsl, Renderer};
use crate::render::shaders;
use crate::resources;
use byteorder::{NativeEndian, WriteBytesExt};
use image::GenericImageView;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::render::ui::{UIState, UIText};
use crate::ui;
use crate::ui::{Container, ImageRef, FormattedRef, VAttach, HAttach, Text};
use crate::screen::settings_menu::UIElements;
use crate::screen::{Screen, AudioSettingsMenu};
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::rngs::ThreadRng;
use rand::Rng;
use leafish_protocol::format::{TextComponent, Component};
use std::rc::Rc;
use std::cell::RefCell;

// Textures can be found at: assets/minecraft/textures/gui/icons.png

pub struct HudContext {

    pub enabled: bool,
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
    dirty_breath: bool

}

impl HudContext {

    pub fn new() -> Self {
        HudContext {
            enabled: true,
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
            breath: 0/*-1*/, // -1 = disabled (not under water) | 1 bubble = 30 | +2 = broken bubble -- -1 is causing crashes when attempting to join servers!
            dirty_breath: false
        }
    }
    // TODO: Implement effects!

    pub fn update_health_and_food(&mut self, health: f32, food: u8, saturation: u8) {
        let start = SystemTime::now();
        let time = start
            .duration_since(UNIX_EPOCH).unwrap().as_millis();
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

}

pub struct Hud {

    last_enabled: bool,
    elements: Vec<ImageRef>,
    health_elements: Vec<ImageRef>,
    armor_elements: Vec<ImageRef>,
    food_elements: Vec<ImageRef>,
    breath_elements: Vec<ImageRef>,
    exp_elements: Vec<ImageRef>,
    exp_text_elements: Vec<Rc<RefCell<Text>>>,
    hud_context: Arc<RwLock<HudContext>>,
    random: ThreadRng,

}

impl Hud {
    
    pub fn new(hud_context: Arc<RwLock<HudContext>>) -> Self {
        Hud {
            last_enabled: true,
            elements: vec![],
            health_elements: vec![],
            armor_elements: vec![],
            food_elements: vec![],
            breath_elements: vec![],
            exp_elements: vec![],
            exp_text_elements: vec![],
            hud_context: hud_context.clone(),
            random: rand::thread_rng(),
        }
    }

    fn max(first: f64, second: f64) -> f64 {
        if first > second {
            first
        }else {
            second
        }
    }
    
}

impl Screen for Hud {

    fn on_active(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        self.render_health(renderer, ui_container);
        self.render_armor(renderer, ui_container);
        self.render_slots(renderer, ui_container);
        self.render_exp(renderer, ui_container);
        self.render_crosshair(renderer, ui_container);
        self.render_food(renderer, ui_container);
        self.render_breath(renderer, ui_container);
    }

    fn on_deactive(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        self.elements.clear();
        self.health_elements.clear();
        self.exp_elements.clear();
        self.exp_text_elements.clear();
        self.food_elements.clear();
        self.armor_elements.clear();
        self.breath_elements.clear();
    }

    fn tick(
        &mut self,
        _delta: f64,
        renderer: &mut render::Renderer,
        ui_container: &mut ui::Container,
    ) -> Option<Box<dyn Screen>> {
        if !self.hud_context.clone().read().unwrap().enabled && self.last_enabled {
            self.on_deactive(renderer, ui_container);
            self.last_enabled = false;
            return None;
        }
        if self.hud_context.clone().read().unwrap().enabled && !self.last_enabled {
            self.on_active(renderer, ui_container);
            self.last_enabled = true;
            return None;
        }
        if self.hud_context.clone().read().unwrap().dirty_health {
            self.health_elements.clear();
            self.render_health(renderer, ui_container);
        }
        if self.hud_context.clone().read().unwrap().dirty_armor {
            self.armor_elements.clear();
            self.render_armor(renderer, ui_container);
        }
        if self.hud_context.clone().read().unwrap().dirty_food {
            self.food_elements.clear();
            self.render_food(renderer, ui_container);
        }
        if self.hud_context.clone().read().unwrap().dirty_exp {
            self.exp_elements.clear();
            self.exp_text_elements.clear();
            self.render_exp(renderer, ui_container);
        }
        if self.hud_context.clone().read().unwrap().dirty_breath {
            self.breath_elements.clear();
            self.render_breath(renderer, ui_container);
        }
        None
    }


    fn is_closable(&self) -> bool {
        false
    }

}

impl Hud {

    pub fn icon_scale(renderer: &Renderer) -> f32 {
        let icon_scale = if renderer.height > 500 {
            renderer.height as f32 / 38.88
        }else {
            renderer.height as f32 / 27.5/*25.0*/
        };
        icon_scale
    }

    fn render_health(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let hud_context = self.hud_context.clone();
        let hud_context = hud_context.read().unwrap();
        let icon_scale = Hud::icon_scale(renderer);
        let x_offset = icon_scale as f64 / 9.0 * 182.0 / 2.0 * -1.0 + icon_scale as f64 / 2.0;
        let y_offset = icon_scale as f64 / 9.0 * 31.0;
        let hp = hud_context.health.ceil();
        let max_health = hud_context.max_health;
        let absorbtion = hud_context.absorbtion;
        let last_health = hud_context.last_health;
        let mut tmp_absorbtion = absorbtion;
        let mut regen_animation = -1; // TODO: Implement regen animation!
        let updated_health = false; // whether health updated recently or not
        // TODO: Implement updated health animation!
        let updated_offset = if updated_health {
            9.0
        } else {
            0.0
        };
        let hardcore_offset = if hud_context.hardcore {
            5.0
        } else {
            0.0
        };
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
            let heart_rows = ((heart + 1) as f32 / 10.0).ceil() - 1.0;
            let x = x_offset as f32 + (heart as f32) % 10.0 * (icon_scale / 9.0 * 8.0);
            let mut y = y_offset as f32 + (heart_rows * (icon_scale + (icon_scale / 9.0 * 1.0)));

            if heart == regen_animation {
                // This moves the hearts down when the regeneration effect is active
                y -= icon_scale / 9.0 * 2.0;
            }

            if hp <= 4.0 {
                // Creates the jittery effect when player has less than 2.5 hearts
                y += icon_scale / 9.0 * (self.random.gen_range(0..2) as f32);
                redirty_health = true;
            }

            let image = ui::ImageBuilder::new()
                .texture_coords(((16.0 + updated_offset) as f64 / 256.0, (9.0 * hardcore_offset) as f64 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                .position(x as f64, y as f64)
                .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                .size(icon_scale as f64, icon_scale as f64)
                .texture("minecraft:gui/icons")
                .create(ui_container);
            self.health_elements.push(image);

            if updated_health {
                if heart as f32 * 2.0 + 1.0 < last_health {
                    let image = ui::ImageBuilder::new()
                        .texture_coords(((texture_offset + 54) as f64 / 256.0, (9.0 * hardcore_offset) as f64 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                        .position(x as f64, y as f64)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale as f64, icon_scale as f64)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                } else if heart as f32 * 2.0 + 1.0 == last_health {
                    let image = ui::ImageBuilder::new()
                        .texture_coords(((texture_offset + 63) as f64 / 256.0, (9.0 * hardcore_offset) as f64 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                        .position(x as f64, y as f64)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale as f64, icon_scale as f64)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                }
            }

            if tmp_absorbtion > 0.0 {
                if tmp_absorbtion == absorbtion && absorbtion % 2.0 == 1.0 {
                    let image = ui::ImageBuilder::new()
                        .texture_coords(((texture_offset + 153) as f64 / 256.0, (9.0 * hardcore_offset) as f64 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                        .position(x as f64, y as f64)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale as f64, icon_scale as f64)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);

                } else {
                    let image = ui::ImageBuilder::new()
                        .texture_coords(((texture_offset + 144) as f64 / 256.0, (9.0 * hardcore_offset) as f64 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                        .position(x as f64, y as f64)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale as f64, icon_scale as f64)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                }

                tmp_absorbtion -= 2.0;
            } else {
                if heart * 2 + 1 < hp as isize {
                    let image = ui::ImageBuilder::new()
                        .texture_coords(((texture_offset + 36) as f64 / 256.0, (9.0 * hardcore_offset) as f64 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                        .position(x as f64, y as f64)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale as f64, icon_scale as f64)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                }

                if heart * 2 + 1 == hp as isize {
                    let image = ui::ImageBuilder::new()
                        .texture_coords(((texture_offset + 45) as f64 / 256.0, (9.0 * hardcore_offset) as f64 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                        .position(x as f64, y as f64)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale as f64, icon_scale as f64)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.health_elements.push(image);
                }
            }
        }
        if !redirty_health {
            self.hud_context.write().unwrap().dirty_health = false;
        }
    }

    fn render_armor(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let armor = self.hud_context.clone().read().unwrap().armor;
        let icon_scale = Hud::icon_scale(renderer);
        let x_offset = icon_scale as f64 / 9.0 * 182.0 / 2.0 * -1.0 + icon_scale as f64 / 2.0;
        let y_offset = icon_scale as f64 / 9.0 * 31.0;
        let max_health = self.hud_context.clone().read().unwrap().max_health;
        let absorbtion = self.hud_context.clone().read().unwrap().absorbtion;
        let icon_bars = (((max_health + absorbtion) / 2.0 / 10.0) as f64).ceil();

        if armor > 0 {
            for i in 0..10 {
                let x = x_offset as f32 + i as f32 * (icon_scale / 9.0 * 8.0);
                let y = y_offset as f32 + (icon_bars as f32 * (icon_scale + (icon_scale / 9.0 * 1.0)));
                let texture_offset = if i * 2 + 1 < armor {
                    34.0
                } else if i * 2 + 1 == armor {
                    25.0
                } else {
                    16.0
                };
                let image = ui::ImageBuilder::new()
                    .texture_coords((texture_offset / 256.0, 9.0 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                    .position(x as f64, y as f64)
                    .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                    .size(icon_scale as f64, icon_scale as f64)
                    .texture("minecraft:gui/icons")
                    .create(ui_container);
                self.armor_elements.push(image);
            }
        }
        self.hud_context.write().unwrap().dirty_armor = false;
    }

    fn render_food(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer) as f64;
        let hud_context = self.hud_context.clone();
        let hud_context = hud_context.read().unwrap();
        let food = hud_context.food;
        let last_food = hud_context.last_food;
        let x_offset = icon_scale as f64 / 9.0 * 182.0 / 2.0 + icon_scale as f64 / 2.0;
        let y_offset = icon_scale as f64 / 9.0 * 31.0;

        let mut l7 = 16.0;
        let mut j8 = 0.0;

        if hud_context.hunger {
            l7 += 36.0;
            j8 = 13.0;
        }

        drop(hud_context);

        for i in 0..10 {
            let x = x_offset - i as f64 * (icon_scale / 9.0 * 8.0) - icon_scale;
            let image = ui::ImageBuilder::new()
                .texture_coords(((16.0 + j8 * 9.0) / 256.0, 27.0 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                .position(x as f64, y_offset as f64)
                .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                .size(icon_scale as f64, icon_scale as f64)
                .texture("minecraft:gui/icons")
                .create(ui_container);
            self.food_elements.push(image);

            if i * 2 + 1 < food {
                let image = ui::ImageBuilder::new()
                    .texture_coords(((l7 + 36.0) / 256.0, 27.0 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                    .position(x as f64, y_offset as f64)
                    .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                    .size(icon_scale as f64, icon_scale as f64)
                    .texture("minecraft:gui/icons")
                    .create(ui_container);
                self.food_elements.push(image);
            } else if i * 2 + 1 == food {
                let image = ui::ImageBuilder::new()
                    .texture_coords(((l7 + 45.0) / 256.0, 27.0 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                    .position(x as f64, y_offset as f64)
                    .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                    .size(icon_scale as f64, icon_scale as f64)
                    .texture("minecraft:gui/icons")
                    .create(ui_container);
                self.food_elements.push(image);
            }
        }
        self.hud_context.write().unwrap().dirty_food = false;
    }

    fn render_exp(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer) as f64;
        let y_offset = icon_scale / 9.0 * 25.0;
        let hud_context = self.hud_context.clone();
        let hud_context = hud_context.read().unwrap();
        let max_exp = if hud_context.exp_level >= 30 {
            112 + (hud_context.exp_level - 30) * 9
        }else if hud_context.exp_level >= 15 {
            37 + (hud_context.exp_level - 15) * 5
        }else {
            7 + hud_context.exp_level * 2
        };
        if max_exp > 0 {
            let image = ui::ImageBuilder::new()
                .texture_coords((0.0 / 256.0, 64.0 / 256.0, 182.0 / 256.0, 5.0 / 256.0))
                .position(0.0 as f64, y_offset as f64)
                .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                .size(icon_scale / 9.0 * 182.0, icon_scale / 9.0 * 5.0)
                .texture("minecraft:gui/icons")
                .create(ui_container);
            self.exp_elements.push(image);

            let scaled_length = hud_context.exp * 182.0;
            if scaled_length > 0.0 {
                let shift = icon_scale / 9.0 * (((182.0) - scaled_length as f64) / 2.0);
                let image = ui::ImageBuilder::new()
                    .texture_coords((0.0 / 256.0, 69.0 / 256.0, scaled_length as f64 / 256.0, 5.0 / 256.0))
                    .position(shift * -1.0, y_offset as f64)
                    .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                    .size(icon_scale / 9.0 * scaled_length as f64, icon_scale / 9.0 * 5.0)
                    .texture("minecraft:gui/icons")
                    .create(ui_container);
                self.exp_elements.push(image);
            }
        }
        if hud_context.exp_level > 0 {
            let level_str = format!("{}", hud_context.exp_level);
            let scale = icon_scale / 9.0 / 2.0;
            let y = icon_scale / 9.0 * 27.0;
            self.exp_text_elements.push(ui::TextBuilder::new()
                .alignment(VAttach::Bottom, HAttach::Center)
                .scale_x(scale)
                .scale_y(scale)
                .position((icon_scale / 9.0 * 1.0), y)
                .text(&level_str)
                .colour((0, 0, 0, 255))
                .shadow(false)
                .create(ui_container));
            self.exp_text_elements.push(ui::TextBuilder::new()
                .alignment(VAttach::Bottom, HAttach::Center)
                .scale_x(scale)
                .scale_y(scale)
                .position(-(icon_scale / 9.0 * 1.0), y)
                .text(&level_str)
                .colour((0, 0, 0, 1))
                .shadow(false)
                .create(ui_container));
            self.exp_text_elements.push(ui::TextBuilder::new()
                .alignment(VAttach::Bottom, HAttach::Center)
                .scale_x(scale)
                .scale_y(scale)
                .position(0.0, y + (icon_scale / 9.0 * 1.0))
                .text(&level_str)
                .colour((0, 0, 0, 255))
                .shadow(false)
                .create(ui_container));
            self.exp_text_elements.push(ui::TextBuilder::new()
                .alignment(VAttach::Bottom, HAttach::Center)
                .scale_x(scale)
                .scale_y(scale)
                .position(0.0, y - (icon_scale / 9.0 * 1.0))
                .text(&level_str)
                .colour((0, 0, 0, 255))
                .shadow(false)
                .create(ui_container));
            self.exp_text_elements.push(ui::TextBuilder::new()
                .alignment(VAttach::Bottom, HAttach::Center)
                .scale_x(scale)
                .scale_y(scale)
                .position(0.0, y)
                .text(&level_str)
                .colour((128, 255, 32, 255))
                .shadow(false)
                .create(ui_container));
        }
        drop(hud_context);
        self.hud_context.write().unwrap().dirty_exp = false;
    }

    fn render_slots(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer) as f64;
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 256.0, 0.0 / 256.0, 182.0 / 256.0, 22.0 / 256.0))
            .position(0.0, 0.0)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
            .size(icon_scale / 9.0 * 182.0, icon_scale / 9.0 * 22.0)
            .texture("minecraft:gui/widgets")
            .create(ui_container);
        self.elements.push(image);
    }

    fn render_scoreboard(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {

    }

    fn render_title(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {

    }

    fn render_item(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {

    }

    fn render_crosshair(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer) as f64;
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 256.0, 0.0 / 256.0, 16.0 / 256.0, 16.0 / 256.0))
            .position(0.0, 0.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .size(icon_scale / 9.0 * 16.0, icon_scale / 9.0 * 16.0)
            .texture("minecraft:gui/icons")
            .create(ui_container);
        self.elements.push(image);
    }

    fn render_breath(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let hud_context = self.hud_context.clone();
        let hud_context = hud_context.read().unwrap();

        if hud_context.breath != -1 { // Whether the player is under water or not.
            let breath = hud_context.breath as f64;
            drop(hud_context);
            let bubbles = ((breath - 2.0) * 10.0 / 300.0).ceil();
            let broken_bubbles = (breath * 10.0 / 300.0).ceil() - bubbles;

            let icon_scale = Hud::icon_scale(renderer) as f64;
            let y_offset = icon_scale / 9.0 * 41.0;
            let x_offset = icon_scale / 9.0 * 182.0 / 2.0 + icon_scale / 2.0;

            for i in 0..bubbles as i32 + broken_bubbles as i32 {
                let x = x_offset - i as f64 * (icon_scale / 9.0 * 8.0) - icon_scale;
                if i < (bubbles as i32) {
                    // normal bubble
                    let image = ui::ImageBuilder::new()
                        .texture_coords((16.0 / 256.0, 18.0 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                        .position(x, y_offset)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale / 9.0 * 9.0, icon_scale / 9.0 * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.elements.push(image);
                } else {
                    // broken bubble
                    let image = ui::ImageBuilder::new()
                        .texture_coords((25.0 / 256.0, 18.0 / 256.0, 9.0 / 256.0, 9.0 / 256.0))
                        .position(x, y_offset)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                        .size(icon_scale / 9.0 * 9.0, icon_scale / 9.0 * 9.0)
                        .texture("minecraft:gui/icons")
                        .create(ui_container);
                    self.elements.push(image);
                }
            }
        }
        self.hud_context.write().unwrap().dirty_breath = false;
    }

}