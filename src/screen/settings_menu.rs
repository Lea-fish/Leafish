// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::console;
use crate::render;
use crate::settings;
use crate::ui;

use crate::screen::{Screen, ScreenSystem};
use std::rc::Rc;
use std::sync::Arc;

pub struct UIElements {
    background: ui::ImageRef,
    _buttons: Vec<ui::ButtonRef>,
}

pub struct SettingsMenu {
    _vars: Rc<console::Vars>,
    elements: Option<UIElements>,
    show_disconnect_button: bool,
}

impl Clone for SettingsMenu {
    fn clone(&self) -> Self {
        SettingsMenu {
            _vars: self._vars.clone(),
            elements: None,
            show_disconnect_button: self.show_disconnect_button,
        }
    }
}

impl SettingsMenu {
    pub fn new(vars: Rc<console::Vars>, show_disconnect_button: bool) -> Self {
        SettingsMenu {
            _vars: vars,
            elements: None,
            show_disconnect_button,
        }
    }
}

impl super::Screen for SettingsMenu {
    fn on_active(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<render::Renderer>,
        ui_container: &mut ui::Container,
    ) {
        let background = ui::ImageBuilder::new()
            .texture("leafish:solid")
            .position(0.0, 0.0)
            .size(854.0, 480.0)
            .colour((0, 0, 0, 100))
            .create(ui_container);

        let mut buttons = vec![];

        // From top and down
        let audio_settings = ui::ButtonBuilder::new()
            .position(-160.0, -50.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut audio_settings = audio_settings.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Audio settings...")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *audio_settings);
            audio_settings.add_text(txt);
            audio_settings.add_click_func(|_, game| {
                game.screen_sys
                    .clone()
                    .add_screen(Box::new(AudioSettingsMenu::new(game.vars.clone())));
                true
            });
        }
        buttons.push(audio_settings);

        let video_settings = ui::ButtonBuilder::new()
            .position(160.0, -50.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut video_settings = video_settings.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Video settings...")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *video_settings);
            video_settings.add_text(txt);
            video_settings.add_click_func(|_, game| {
                game.screen_sys
                    .clone()
                    .add_screen(Box::new(VideoSettingsMenu::new(game.vars.clone())));
                true
            });
        }
        buttons.push(video_settings);

        let controls_settings = ui::ButtonBuilder::new()
            .position(160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut controls_settings = controls_settings.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Controls...")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *controls_settings);
            controls_settings.add_text(txt);
        }
        buttons.push(controls_settings);

        let lang_settings = ui::ButtonBuilder::new()
            .position(-160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut lang_settings = lang_settings.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Language...")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *lang_settings);
            lang_settings.add_text(txt);
        }
        buttons.push(lang_settings);

        let skin_settings = ui::ButtonBuilder::new()
            .position(160.0, -100.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut skin_settings = skin_settings.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Skin Customization...")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *skin_settings);
            skin_settings.add_text(txt);
            skin_settings.add_click_func(|_, game| {
                game.screen_sys
                    .clone()
                    .add_screen(Box::new(SkinSettingsMenu::new(game.vars.clone())));
                true
            });
        }
        buttons.push(skin_settings);

        // Center bottom items
        let done_button = ui::ButtonBuilder::new()
            .position(0.0, 50.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut done_button = done_button.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Done")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *done_button);
            done_button.add_text(txt);
            done_button.add_click_func(|_, game| {
                game.screen_sys.clone().pop_screen();
                true
            });
        }
        buttons.push(done_button);

        if self.show_disconnect_button {
            let disconnect_button = ui::ButtonBuilder::new()
                .position(0.0, 100.0)
                .size(300.0, 40.0)
                .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
                .create(ui_container);
            {
                let mut disconnect_button = disconnect_button.borrow_mut();
                let txt = ui::TextBuilder::new()
                    .text("Disconnect")
                    .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                    .attach(&mut *disconnect_button);
                disconnect_button.add_text(txt);
                disconnect_button.add_click_func(|_, game| {
                    game.server.as_ref().unwrap().disconnect(None);
                    game.screen_sys.clone().pop_screen();
                    game.screen_sys
                        .clone()
                        .replace_screen(Box::new(super::ServerList::new(None)));
                    true
                });
            }
            buttons.push(disconnect_button);
        }

        self.elements = Some(UIElements {
            background,
            _buttons: buttons,
        });
    }

    fn on_deactive(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<render::Renderer>,
        _ui_container: &mut ui::Container,
    ) {
        self.elements = None;
    }

    // Called every frame the screen is active
    fn tick(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<render::Renderer>,
        ui_container: &mut ui::Container,
        _delta: f64,
    ) {
        let elements = self.elements.as_mut().unwrap();
        {
            let mode = ui_container.mode;
            let mut background = elements.background.borrow_mut();
            background.width = match mode {
                ui::Mode::Unscaled(scale) => 854.0 / scale,
                ui::Mode::Scaled => renderer.screen_data.read().width as f64,
            };
            background.height = match mode {
                ui::Mode::Unscaled(scale) => 480.0 / scale,
                ui::Mode::Scaled => renderer.screen_data.read().height as f64,
            };
        }
    }

    // Events
    fn on_scroll(&mut self, _x: f64, _y: f64) {}

    fn is_closable(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}

pub struct VideoSettingsMenu {
    vars: Rc<console::Vars>,
    elements: Option<UIElements>,
}

impl Clone for VideoSettingsMenu {
    fn clone(&self) -> Self {
        VideoSettingsMenu {
            vars: self.vars.clone(),
            elements: None,
        }
    }
}

impl VideoSettingsMenu {
    pub fn new(vars: Rc<console::Vars>) -> Self {
        VideoSettingsMenu {
            vars,
            elements: None,
        }
    }
}

impl super::Screen for VideoSettingsMenu {
    fn on_active(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<render::Renderer>,
        ui_container: &mut ui::Container,
    ) {
        let background = ui::ImageBuilder::new()
            .texture("leafish:solid")
            .position(0.0, 0.0)
            .size(854.0, 480.0)
            .colour((0, 0, 0, 100))
            .create(ui_container);

        let mut buttons = vec![];

        // Load defaults
        let r_max_fps = *self.vars.get(settings::R_MAX_FPS);
        let r_fov = *self.vars.get(settings::R_FOV);
        let r_vsync = *self.vars.get(settings::R_VSYNC);

        // Setting buttons
        // TODO: Slider
        let fov_setting = ui::ButtonBuilder::new()
            .position(160.0, -50.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut fov_setting = fov_setting.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text(format!(
                    "FOV: {}",
                    match r_fov {
                        90 => "Normal".into(),
                        110 => "Quake pro".into(),
                        val => val.to_string(),
                    }
                ))
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *fov_setting);
            fov_setting.add_text(txt);
        }
        buttons.push(fov_setting);

        let vsync_setting = ui::ButtonBuilder::new()
            .position(-160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut vsync_setting = vsync_setting.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text(format!(
                    "VSync: {}",
                    if r_vsync { "Enabled" } else { "Disabled" }
                ))
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *vsync_setting);
            let txt_vsync = txt.clone();
            vsync_setting.add_text(txt);
            vsync_setting.add_click_func(move |_, game| {
                let r_vsync = !*game.vars.get(settings::R_VSYNC);
                txt_vsync.borrow_mut().text =
                    format!("VSync: {}", if r_vsync { "Enabled" } else { "Disabled" });
                game.vars.set(settings::R_VSYNC, r_vsync);
                true
            });
        }
        buttons.push(vsync_setting);

        // TODO: Slider
        let fps_setting = ui::ButtonBuilder::new()
            .position(160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut fps_setting = fps_setting.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text(format!(
                    "FPS cap: {}",
                    match r_max_fps {
                        0 => "Unlimited".into(),
                        val => val.to_string(),
                    }
                ))
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *fps_setting);
            fps_setting.add_text(txt);
        }
        buttons.push(fps_setting);

        let done_button = ui::ButtonBuilder::new()
            .position(0.0, 50.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut done_button = done_button.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Done")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *done_button);
            done_button.add_text(txt);
            done_button.add_click_func(|_, game| {
                game.screen_sys.clone().pop_screen();
                true
            });
        }
        buttons.push(done_button);
        self.elements = Some(UIElements {
            background,
            _buttons: buttons,
        });
    }
    fn on_deactive(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<render::Renderer>,
        _ui_container: &mut ui::Container,
    ) {
        self.elements = None;
    }

    // Called every frame the screen is active
    fn tick(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<render::Renderer>,
        ui_container: &mut ui::Container,
        _delta: f64,
    ) {
        let elements = self.elements.as_mut().unwrap();
        {
            let mode = ui_container.mode;
            let mut background = elements.background.borrow_mut();
            background.width = match mode {
                ui::Mode::Unscaled(scale) => 854.0 / scale,
                ui::Mode::Scaled => renderer.screen_data.read().width as f64,
            };
            background.height = match mode {
                ui::Mode::Unscaled(scale) => 480.0 / scale,
                ui::Mode::Scaled => renderer.screen_data.read().height as f64,
            };
        }
    }

    // Events
    fn on_scroll(&mut self, _x: f64, _y: f64) {}

    fn is_closable(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}

pub struct AudioSettingsMenu {
    _vars: Rc<console::Vars>,
    elements: Option<UIElements>,
}

impl Clone for AudioSettingsMenu {
    fn clone(&self) -> Self {
        AudioSettingsMenu {
            _vars: self._vars.clone(),
            elements: None,
        }
    }
}

impl AudioSettingsMenu {
    pub fn new(vars: Rc<console::Vars>) -> AudioSettingsMenu {
        AudioSettingsMenu {
            _vars: vars,
            elements: None,
        }
    }
}

impl super::Screen for AudioSettingsMenu {
    fn on_active(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<render::Renderer>,
        ui_container: &mut ui::Container,
    ) {
        let background = ui::ImageBuilder::new()
            .texture("leafish:solid")
            .position(0.0, 0.0)
            .size(854.0, 480.0)
            .colour((0, 0, 0, 100))
            .create(ui_container);

        let mut buttons = vec![];

        // TODO

        let done_button = ui::ButtonBuilder::new()
            .position(0.0, 50.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut done_button = done_button.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Done")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *done_button);
            done_button.add_text(txt);
            done_button.add_click_func(|_, game| {
                game.screen_sys.clone().pop_screen();
                true
            });
        }
        buttons.push(done_button);

        self.elements = Some(UIElements {
            background,
            _buttons: buttons,
        });
    }
    fn on_deactive(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<render::Renderer>,
        _ui_container: &mut ui::Container,
    ) {
        self.elements = None;
    }

    // Called every frame the screen is active
    fn tick(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<render::Renderer>,
        ui_container: &mut ui::Container,
        _delta: f64,
    ) {
        let elements = self.elements.as_mut().unwrap();
        {
            let mode = ui_container.mode;
            let mut background = elements.background.borrow_mut();
            background.width = match mode {
                ui::Mode::Unscaled(scale) => 854.0 / scale,
                ui::Mode::Scaled => renderer.screen_data.read().width as f64,
            };
            background.height = match mode {
                ui::Mode::Unscaled(scale) => 480.0 / scale,
                ui::Mode::Scaled => renderer.screen_data.read().height as f64,
            };
        }
    }

    // Events
    fn on_scroll(&mut self, _x: f64, _y: f64) {}

    fn is_closable(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}

pub struct SkinSettingsMenu {
    vars: Rc<console::Vars>,
    elements: Option<UIElements>,
}

impl Clone for SkinSettingsMenu {
    fn clone(&self) -> Self {
        SkinSettingsMenu {
            vars: self.vars.clone(),
            elements: None,
        }
    }
}

impl SkinSettingsMenu {
    pub fn new(vars: Rc<console::Vars>) -> Self {
        SkinSettingsMenu {
            vars,
            elements: None,
        }
    }
}

impl super::Screen for SkinSettingsMenu {
    fn on_active(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<render::Renderer>,
        ui_container: &mut ui::Container,
    ) {
        let background = ui::ImageBuilder::new()
            .texture("leafish:solid")
            .position(0.0, 0.0)
            .size(854.0, 480.0)
            .colour((0, 0, 0, 100))
            .create(ui_container);

        let mut buttons = vec![];

        // Load defaults
        let s_hat = *self.vars.get(settings::S_HAT);
        let _s_jacket = *self.vars.get(settings::S_JACKET);
        let _s_cape = *self.vars.get(settings::S_CAPE);
        let _s_right_sleeve = *self.vars.get(settings::S_RIGHT_SLEEVE);
        let _s_left_sleeve = *self.vars.get(settings::S_LEFT_SLEEVE);
        let _s_right_pants = *self.vars.get(settings::S_RIGHT_PANTS);
        let _s_left_pants = *self.vars.get(settings::S_LEFT_PANTS);

        // Setting buttons
        let hat_setting = ui::ButtonBuilder::new()
            .position(160.0, -50.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut hat_setting = hat_setting.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text(format!(
                    "Hat: {}",
                    match s_hat {
                        true => "On",
                        false => "Off",
                    }
                ))
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *hat_setting);
            let txt_hat = txt.clone();
            hat_setting.add_text(txt);
            hat_setting.add_click_func(move |_, game| {
                let s_hat = !*game.vars.get(settings::S_HAT);
                txt_hat.borrow_mut().text = format!(
                    "Hat: {}",
                    match s_hat {
                        true => "On",
                        false => "Off",
                    }
                );
                game.vars.set(settings::S_HAT, s_hat);
                false
            });
        }
        buttons.push(hat_setting);

        /*
        let vsync_setting = ui::ButtonBuilder::new()
            .position(-160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut vsync_setting = vsync_setting.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text(format!(
                    "VSync: {}",
                    if r_vsync { "Enabled" } else { "Disabled" }
                ))
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *vsync_setting);
            let txt_vsync = txt.clone();
            vsync_setting.add_text(txt);
            vsync_setting.add_click_func(move |_, game| {
                let r_vsync = !*game.vars.get(settings::R_VSYNC);
                txt_vsync.borrow_mut().text =
                    format!("VSync: {}", if r_vsync { "Enabled" } else { "Disabled" });
                game.vars.set(settings::R_VSYNC, r_vsync);
                true
            });
        }
        buttons.push(vsync_setting);

        // TODO: Slider
        let fps_setting = ui::ButtonBuilder::new()
            .position(160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut fps_setting = fps_setting.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text(format!(
                    "FPS cap: {}",
                    match r_max_fps {
                        0 => "Unlimited".into(),
                        val => val.to_string(),
                    }
                ))
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *fps_setting);
            fps_setting.add_text(txt);
        }
        buttons.push(fps_setting);*/

        let done_button = ui::ButtonBuilder::new()
            .position(0.0, 50.0)
            .size(300.0, 40.0)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut done_button = done_button.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Done")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *done_button);
            done_button.add_text(txt);
            done_button.add_click_func(|_, game| {
                game.screen_sys.clone().pop_screen();
                true
            });
        }
        buttons.push(done_button);
        self.elements = Some(UIElements {
            background,
            _buttons: buttons,
        });
    }
    fn on_deactive(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<render::Renderer>,
        _ui_container: &mut ui::Container,
    ) {
        self.elements = None;
    }

    // Called every frame the screen is active
    fn tick(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<render::Renderer>,
        ui_container: &mut ui::Container,
        _delta: f64,
    ) {
        let elements = self.elements.as_mut().unwrap();
        {
            let mode = ui_container.mode;
            let mut background = elements.background.borrow_mut();
            background.width = match mode {
                ui::Mode::Unscaled(scale) => 854.0 / scale,
                ui::Mode::Scaled => renderer.screen_data.read().width as f64,
            };
            background.height = match mode {
                ui::Mode::Unscaled(scale) => 480.0 / scale,
                ui::Mode::Scaled => renderer.screen_data.read().height as f64,
            };
        }
    }

    // Events
    fn on_scroll(&mut self, _x: f64, _y: f64) {}

    fn is_closable(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}
