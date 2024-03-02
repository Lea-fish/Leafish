use crate::render::Renderer;
use crate::screen::{Screen, ScreenSystem};
use crate::settings::{SettingStore, SettingType};
use crate::ui;
use crate::ui::Container;
use std::rc::Rc;
use std::sync::Arc;

pub struct Background {
    background: Option<ui::ImageRef>,
    settings: Rc<SettingStore>,
    screen_sys: Arc<ScreenSystem>,
    active: bool,
    delay: f64,
    last_path: String,
}

impl Clone for Background {
    fn clone(&self) -> Self {
        Self::new(self.settings.clone(), self.screen_sys.clone())
    }
}

impl Background {
    pub fn new(settings: Rc<SettingStore>, screen_sys: Arc<ScreenSystem>) -> Self {
        Self {
            background: None,
            settings,
            screen_sys,
            active: false,
            delay: 0.0,
            last_path: "".to_string(),
        }
    }
}

impl Screen for Background {
    fn init(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        let path = self.settings.get_string(SettingType::BackgroundImage);
        self.last_path = path.clone();
        let background =
            if Renderer::get_texture_optional(renderer.get_textures_ref(), &format!("#{}", path))
                .is_some()
            {
                Some(
                    ui::ImageBuilder::new()
                        .draw_index(i16::MIN as isize)
                        .texture(&*format!(
                            "#{}",
                            self.settings.get_string(SettingType::BackgroundImage)
                        ))
                        .size(
                            renderer.screen_data.read().safe_width as f64,
                            renderer.screen_data.read().safe_height as f64,
                        )
                        .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                        .create(ui_container),
                )
            } else {
                None
            };
        self.active = true;
        self.background = background;
    }

    fn deinit(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
        self.background.take();
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
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        delta: f64,
    ) {
        self.delay += delta;
        if self.delay >= 0.1 {
            self.delay = 0.0;
            let hide = self.screen_sys.is_any_ingame();
            if self.active {
                if hide {
                    self.active = false;
                    self.deinit(screen_sys, renderer, ui_container);
                    return;
                }
            } else if !hide {
                self.init(screen_sys, renderer, ui_container);
                return;
            }
            let curr_path = self.settings.get_string(SettingType::BackgroundImage);
            if !self.last_path.eq(&curr_path) {
                self.last_path = curr_path;
                self.deinit(screen_sys, renderer.clone(), ui_container);
                self.init(screen_sys, renderer, ui_container);
            }
        }
    }

    fn on_resize(
        &mut self,
        screen_sys: &ScreenSystem,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        self.deinit(screen_sys, renderer.clone(), ui_container);
        self.init(screen_sys, renderer, ui_container);
    }

    fn is_tick_always(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}
