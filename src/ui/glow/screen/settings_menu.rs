use crate::settings::SettingStore;

use crate::ui::glow::ctx;
use crate::ui::glow::ui::Mode;
use crate::ui::glow::ui::SliderBuilder;
use crate::ui::glow::Renderer;
use crate::ui::glow::ui::ButtonBuilder;
use crate::ui::glow::ui::ButtonRef;
use crate::ui::glow::ui::Container;
use crate::ui::glow::ui::HAttach;
use crate::ui::glow::ui::ImageBuilder;
use crate::ui::glow::ui::ImageRef;
use crate::ui::glow::ui::SliderRef;
use crate::ui::glow::ui::TextBuilder;
use crate::ui::glow::ui::VAttach;
use crate::BoolSetting;
use crate::FloatSetting;
use crate::IntSetting;
use std::sync::Arc;

use super::Screen;
use super::ScreenSystem;

pub struct UIElements {
    background: ImageRef,
    _buttons: Vec<ButtonRef>,
    _sliders: Vec<SliderRef>,
}

pub struct SettingsMenu {
    settings: Arc<SettingStore>,
    elements: Option<UIElements>,
    show_disconnect_button: bool,
}

impl Clone for SettingsMenu {
    fn clone(&self) -> Self {
        Self {
            settings: self.settings.clone(),
            elements: None,
            show_disconnect_button: self.show_disconnect_button,
        }
    }
}

impl SettingsMenu {
    pub fn new(settings: Arc<SettingStore>, show_disconnect_button: bool) -> Self {
        Self {
            settings,
            elements: None,
            show_disconnect_button,
        }
    }
}

impl super::Screen for SettingsMenu {
    fn on_active(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        let background = ImageBuilder::new()
            .texture("leafish:solid")
            .position(0.0, 0.0)
            .size(854.0, 480.0)
            .colour((0, 0, 0, 100))
            .create(ui_container);

        let mut buttons = vec![];

        // From top and down
        let audio_settings = ButtonBuilder::new()
            .position(-160.0, -50.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut audio_settings = audio_settings.borrow_mut();
            let txt = TextBuilder::new()
                .text("Audio settings...")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *audio_settings);
            audio_settings.add_text(txt);
            audio_settings.add_click_func(|_, game| {
                ctx().screen_sys
                    .add_screen(Box::new(AudioSettingsMenu::new(game.settings.clone())));
                true
            });
        }
        buttons.push(audio_settings);

        let video_settings = ButtonBuilder::new()
            .position(160.0, -50.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut video_settings = video_settings.borrow_mut();
            let txt = TextBuilder::new()
                .text("Video settings...")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *video_settings);
            video_settings.add_text(txt);
            video_settings.add_click_func(|_, game| {
                ctx().screen_sys
                    .add_screen(Box::new(VideoSettingsMenu::new(game.settings.clone())));
                true
            });
        }
        buttons.push(video_settings);

        let controls_settings = ButtonBuilder::new()
            .position(160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut controls_settings = controls_settings.borrow_mut();
            let txt = TextBuilder::new()
                .text("Controls...")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *controls_settings);
            controls_settings.add_text(txt);
            controls_settings.add_click_func(|_, game| {
                ctx().screen_sys
                    .add_screen(Box::new(ControlsMenu::new(game.settings.clone())));
                true
            });
        }
        buttons.push(controls_settings);

        let lang_settings = ButtonBuilder::new()
            .position(-160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut lang_settings = lang_settings.borrow_mut();
            let txt = TextBuilder::new()
                .text("Language...")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *lang_settings);
            lang_settings.add_text(txt);
        }
        buttons.push(lang_settings);

        let skin_settings = ButtonBuilder::new()
            .position(160.0, -100.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut skin_settings = skin_settings.borrow_mut();
            let txt = TextBuilder::new()
                .text("Skin Customization...")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *skin_settings);
            skin_settings.add_text(txt);
            skin_settings.add_click_func(|_, game| {
                ctx().screen_sys
                    .clone()
                    .add_screen(Box::new(SkinSettingsMenu::new(game.settings.clone())));
                true
            });
        }
        buttons.push(skin_settings);

        // Center bottom items
        let done_button = ButtonBuilder::new()
            .position(0.0, 50.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Bottom, HAttach::Center)
            .create(ui_container);
        {
            let mut done_button = done_button.borrow_mut();
            let txt = TextBuilder::new()
                .text("Done")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *done_button);
            done_button.add_text(txt);
            done_button.add_click_func(|_, _game| {
                ctx().screen_sys.clone().pop_screen();
                true
            });
        }
        buttons.push(done_button);

        if self.show_disconnect_button {
            let disconnect_button = ButtonBuilder::new()
                .position(0.0, 100.0)
                .size(300.0, 40.0)
                .alignment(VAttach::Bottom, HAttach::Center)
                .create(ui_container);
            {
                let mut disconnect_button = disconnect_button.borrow_mut();
                let txt = TextBuilder::new()
                    .text("Disconnect")
                    .alignment(VAttach::Middle, HAttach::Center)
                    .attach(&mut *disconnect_button);
                disconnect_button.add_text(txt);
                disconnect_button.add_click_func(|_, game| {
                    game.server.load().as_ref().unwrap().disconnect(None);
                    let ctx = ctx();
                    ctx.screen_sys.clone().pop_screen();
                    ctx.screen_sys
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
            _sliders: vec![],
        });
    }

    fn on_deactive(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
        self.elements = None;
    }

    // Called every frame the screen is active
    fn tick(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        _delta: f64,
    ) {
        let elements = self.elements.as_mut().unwrap();
        {
            let mode = ui_container.mode;
            let mut background = elements.background.borrow_mut();
            background.width = match mode {
                Mode::Unscaled(scale) => 854.0 / scale,
                Mode::Scaled => renderer.screen_data.read().width as f64,
            };
            background.height = match mode {
                Mode::Unscaled(scale) => 480.0 / scale,
                Mode::Scaled => renderer.screen_data.read().height as f64,
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
    settings: Arc<SettingStore>,
    elements: Option<UIElements>,
}

impl Clone for VideoSettingsMenu {
    fn clone(&self) -> Self {
        Self {
            settings: self.settings.clone(),
            elements: None,
        }
    }
}

impl VideoSettingsMenu {
    pub fn new(settings: Arc<SettingStore>) -> Self {
        Self {
            settings,
            elements: None,
        }
    }
}

impl super::Screen for VideoSettingsMenu {
    fn on_active(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        let background = ImageBuilder::new()
            .texture("leafish:solid")
            .position(0.0, 0.0)
            .size(854.0, 480.0)
            .colour((0, 0, 0, 100))
            .create(ui_container);

        let mut buttons = vec![];

        // Load defaults
        let r_max_fps = self.settings.get_int(IntSetting::MaxFps);
        let r_fov = self.settings.get_int(IntSetting::FOV);
        let r_vsync = self.settings.get_bool(BoolSetting::Vsync);

        // Setting buttons
        // TODO: Slider
        let fov_setting = ButtonBuilder::new()
            .position(160.0, -50.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut fov_setting = fov_setting.borrow_mut();
            let txt = TextBuilder::new()
                .text(format!(
                    "FOV: {}",
                    match r_fov {
                        90 => "Normal".into(),
                        110 => "Quake pro".into(),
                        val => val.to_string(),
                    }
                ))
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *fov_setting);
            fov_setting.add_text(txt);
        }
        buttons.push(fov_setting);

        let vsync_setting = ButtonBuilder::new()
            .position(-160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut vsync_setting = vsync_setting.borrow_mut();
            let txt = TextBuilder::new()
                .text(format!(
                    "VSync: {}",
                    if r_vsync { "Enabled" } else { "Disabled" }
                ))
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *vsync_setting);
            let txt_vsync = txt.clone();
            vsync_setting.add_text(txt);
            vsync_setting.add_click_func(move |_, game| {
                let r_vsync = !game.settings.get_bool(BoolSetting::Vsync);
                txt_vsync.borrow_mut().text =
                    format!("VSync: {}", if r_vsync { "Enabled" } else { "Disabled" });
                game.settings.set_bool(BoolSetting::Vsync, r_vsync);
                true
            });
        }
        buttons.push(vsync_setting);

        // TODO: Slider
        let fps_setting = ButtonBuilder::new()
            .position(160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut fps_setting = fps_setting.borrow_mut();
            let txt = TextBuilder::new()
                .text(format!(
                    "FPS cap: {}",
                    match r_max_fps {
                        0 => "Unlimited".into(),
                        val => val.to_string(),
                    }
                ))
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *fps_setting);
            fps_setting.add_text(txt);
        }
        buttons.push(fps_setting);

        let done_button = ButtonBuilder::new()
            .position(0.0, 50.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Bottom, HAttach::Center)
            .create(ui_container);
        {
            let mut done_button = done_button.borrow_mut();
            let txt = TextBuilder::new()
                .text("Done")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *done_button);
            done_button.add_text(txt);
            done_button.add_click_func(|_, _game| {
                ctx().screen_sys.pop_screen();
                true
            });
        }
        buttons.push(done_button);
        self.elements = Some(UIElements {
            background,
            _buttons: buttons,
            _sliders: vec![],
        });
    }
    fn on_deactive(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
        self.elements = None;
    }

    // Called every frame the screen is active
    fn tick(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        _delta: f64,
    ) {
        let elements = self.elements.as_mut().unwrap();
        {
            let mode = ui_container.mode;
            let mut background = elements.background.borrow_mut();
            background.width = match mode {
                Mode::Unscaled(scale) => 854.0 / scale,
                Mode::Scaled => renderer.screen_data.read().width as f64,
            };
            background.height = match mode {
                Mode::Unscaled(scale) => 480.0 / scale,
                Mode::Scaled => renderer.screen_data.read().height as f64,
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
    _settings: Arc<SettingStore>,
    elements: Option<UIElements>,
}

impl Clone for AudioSettingsMenu {
    fn clone(&self) -> Self {
        AudioSettingsMenu {
            _settings: self._settings.clone(),
            elements: None,
        }
    }
}

impl AudioSettingsMenu {
    pub fn new(_settings: Arc<SettingStore>) -> AudioSettingsMenu {
        AudioSettingsMenu {
            _settings,
            elements: None,
        }
    }
}

impl super::Screen for AudioSettingsMenu {
    fn on_active(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        let background = ImageBuilder::new()
            .texture("leafish:solid")
            .position(0.0, 0.0)
            .size(854.0, 480.0)
            .colour((0, 0, 0, 100))
            .create(ui_container);

        let mut buttons = vec![];

        // TODO

        let done_button = ButtonBuilder::new()
            .position(0.0, 50.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Bottom, HAttach::Center)
            .create(ui_container);
        {
            let mut done_button = done_button.borrow_mut();
            let txt = TextBuilder::new()
                .text("Done")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *done_button);
            done_button.add_text(txt);
            done_button.add_click_func(|_, _game| {
                ctx().screen_sys.pop_screen();
                true
            });
        }
        buttons.push(done_button);

        self.elements = Some(UIElements {
            background,
            _buttons: buttons,
            _sliders: vec![],
        });
    }
    fn on_deactive(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
        self.elements = None;
    }

    // Called every frame the screen is active
    fn tick(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        _delta: f64,
    ) {
        let elements = self.elements.as_mut().unwrap();
        {
            let mode = ui_container.mode;
            let mut background = elements.background.borrow_mut();
            background.width = match mode {
                Mode::Unscaled(scale) => 854.0 / scale,
                Mode::Scaled => renderer.screen_data.read().width as f64,
            };
            background.height = match mode {
                Mode::Unscaled(scale) => 480.0 / scale,
                Mode::Scaled => renderer.screen_data.read().height as f64,
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
    settings: Arc<SettingStore>,
    elements: Option<UIElements>,
}

impl Clone for SkinSettingsMenu {
    fn clone(&self) -> Self {
        Self {
            settings: self.settings.clone(),
            elements: None,
        }
    }
}

impl SkinSettingsMenu {
    pub fn new(settings: Arc<SettingStore>) -> Self {
        SkinSettingsMenu {
            settings,
            elements: None,
        }
    }
}

impl super::Screen for SkinSettingsMenu {
    fn on_active(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        let background = ImageBuilder::new()
            .texture("leafish:solid")
            .position(0.0, 0.0)
            .size(854.0, 480.0)
            .colour((0, 0, 0, 100))
            .create(ui_container);

        let mut buttons = vec![];

        // Load defaults
        let s_hat = self.settings.get_bool(BoolSetting::HatVisible);
        let _s_jacket = self.settings.get_bool(BoolSetting::JacketVisible);
        let _s_cape = self.settings.get_bool(BoolSetting::CapeVisible);
        let _s_right_sleeve = self.settings.get_bool(BoolSetting::RightSleeveVisible);
        let _s_left_sleeve = self.settings.get_bool(BoolSetting::LeftSleeveVisible);
        let _s_right_pants = self.settings.get_bool(BoolSetting::RightPantsVisible);
        let _s_left_pants = self.settings.get_bool(BoolSetting::LeftPantsVisible);

        // Setting buttons
        let hat_setting = ButtonBuilder::new()
            .position(160.0, -50.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut hat_setting = hat_setting.borrow_mut();
            let txt = TextBuilder::new()
                .text(format!(
                    "Hat: {}",
                    match s_hat {
                        true => "On",
                        false => "Off",
                    }
                ))
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *hat_setting);
            let txt_hat = txt.clone();
            hat_setting.add_text(txt);
            hat_setting.add_click_func(move |_, game| {
                let s_hat = !game.settings.get_bool(BoolSetting::HatVisible);
                txt_hat.borrow_mut().text = format!(
                    "Hat: {}",
                    match s_hat {
                        true => "On",
                        false => "Off",
                    }
                );
                game.settings.set_bool(BoolSetting::HatVisible, s_hat);
                false
            });
        }
        buttons.push(hat_setting);

        /*
        let vsync_setting = ButtonBuilder::new()
            .position(-160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut vsync_setting = vsync_setting.borrow_mut();
            let txt = TextBuilder::new()
                .text(format!(
                    "VSync: {}",
                    if r_vsync { "Enabled" } else { "Disabled" }
                ))
                .alignment(VAttach::Middle, HAttach::Center)
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
        let fps_setting = ButtonBuilder::new()
            .position(160.0, 0.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut fps_setting = fps_setting.borrow_mut();
            let txt = TextBuilder::new()
                .text(format!(
                    "FPS cap: {}",
                    match r_max_fps {
                        0 => "Unlimited".into(),
                        val => val.to_string(),
                    }
                ))
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *fps_setting);
            fps_setting.add_text(txt);
        }
        buttons.push(fps_setting);*/

        let done_button = ButtonBuilder::new()
            .position(0.0, 50.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Bottom, HAttach::Center)
            .create(ui_container);
        {
            let mut done_button = done_button.borrow_mut();
            let txt = TextBuilder::new()
                .text("Done")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *done_button);
            done_button.add_text(txt);
            done_button.add_click_func(|_, _game| {
                ctx().screen_sys.pop_screen();
                true
            });
        }
        buttons.push(done_button);
        self.elements = Some(UIElements {
            background,
            _buttons: buttons,
            _sliders: vec![],
        });
    }
    fn on_deactive(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
        self.elements = None;
    }

    // Called every frame the screen is active
    fn tick(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        _delta: f64,
    ) {
        let elements = self.elements.as_mut().unwrap();
        {
            let mode = ui_container.mode;
            let mut background = elements.background.borrow_mut();
            background.width = match mode {
                Mode::Unscaled(scale) => 854.0 / scale,
                Mode::Scaled => renderer.screen_data.read().width as f64,
            };
            background.height = match mode {
                Mode::Unscaled(scale) => 480.0 / scale,
                Mode::Scaled => renderer.screen_data.read().height as f64,
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

pub struct ControlsMenu {
    settings: Arc<SettingStore>,
    elements: Option<UIElements>,
}

impl Clone for ControlsMenu {
    fn clone(&self) -> Self {
        ControlsMenu {
            settings: self.settings.clone(),
            elements: None,
        }
    }
}

impl ControlsMenu {
    pub fn new(settings: Arc<SettingStore>) -> Self {
        ControlsMenu {
            settings,
            elements: None,
        }
    }
}

impl super::Screen for ControlsMenu {
    fn on_active(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        let mut buttons = vec![];
        let mut sliders = vec![];
        let r_mouse_sens = self.settings.get_float(FloatSetting::MouseSense);

        let background = ImageBuilder::new()
            .texture("leafish:solid")
            .position(0.0, 0.0)
            .size(854.0, 480.0)
            .colour((0, 0, 0, 100))
            .create(ui_container);

        let done_button = ButtonBuilder::new()
            .position(0.0, 50.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Bottom, HAttach::Center)
            .create(ui_container);
        {
            let mut done_button = done_button.borrow_mut();
            let txt = TextBuilder::new()
                .text("Done")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *done_button);
            done_button.add_text(txt);
            done_button.add_click_func(|_, _game| {
                ctx().screen_sys.pop_screen();
                true
            });
        }
        buttons.push(done_button);

        let slider = SliderBuilder::new()
            .position(160.0, -50.0)
            .size(300.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut slider = slider.borrow_mut();
            let txt = TextBuilder::new()
                .text(format!("Mouse Sensetivity: {:.2}x", r_mouse_sens))
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *slider);
            slider.add_text(txt);
            slider.button.as_mut().unwrap().borrow_mut().x = r_mouse_sens * 30.0 - 150.0;
            slider.add_click_func(|this, game| {
                let ctx = ctx();
                let screen_width = ctx.screen_sys.screens.read().last().unwrap().last_width as f64;
                let slider_btn = this.button.as_mut().expect("Slider had no button");
                //update button position
                slider_btn.borrow_mut().x = (ctx.input_ctx.read().last_mouse_x) - screen_width / 2.0 - this.x;
                //update game setting based on button position
                game.settings.set_float(
                    FloatSetting::MouseSense,
                    (slider_btn.borrow().x + 150.0) / 30.0,
                );
                //update text in button
                this.text
                    .as_mut()
                    .expect("Slider had no text")
                    .borrow_mut()
                    .text = format!(
                    "Mouse Sensetivity: {:.2}x",
                    game.settings.get_float(FloatSetting::MouseSense)
                );
                true
            });
        }

        sliders.push(slider);

        self.elements = Some(UIElements {
            background,
            _buttons: buttons,
            _sliders: sliders,
        });
    }

    fn on_deactive(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
        self.elements = None;
    }

    // Called every frame the screen is active
    fn tick(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        _delta: f64,
    ) {
        let elements = self.elements.as_mut().unwrap();
        {
            let mode = ui_container.mode;
            let mut background = elements.background.borrow_mut();
            background.width = match mode {
                Mode::Unscaled(scale) => 854.0 / scale,
                Mode::Scaled => renderer.screen_data.read().width as f64,
            };
            background.height = match mode {
                Mode::Unscaled(scale) => 480.0 / scale,
                Mode::Scaled => renderer.screen_data.read().height as f64,
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
