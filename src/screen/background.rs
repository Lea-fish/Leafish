use crate::{ui, console, Game};
use std::rc::Rc;
use crate::screen::{Screen, ScreenSystem};
use crate::render::Renderer;
use crate::ui::Container;
use crate::settings::BACKGROUND_IMAGE;
use std::sync::Arc;
use glutin::event::VirtualKeyCode;

pub struct Background {

    background: Option<ui::ImageRef>,
    vars: Rc<console::Vars>,
    screen_sys: Arc<ScreenSystem>,
    active: bool,

}

impl Clone for Background {
    fn clone(&self) -> Self {
        Self::new(self.vars.clone(), self.screen_sys.clone())
    }
}

impl Background {

    pub fn new(vars: Rc<console::Vars>, screen_sys: Arc<ScreenSystem>) -> Self {
        Self {
            background: None,
            vars,
            screen_sys,
            active: false,
        }
    }

}

impl Screen for Background {
    fn init(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let background = if Renderer::get_texture_optional(
            renderer.get_textures_ref(),
            &*format!("#{}", self.vars.get(BACKGROUND_IMAGE)),
        )
            .is_some()
        {
            Some(
                ui::ImageBuilder::new()
                    .texture(&*format!("#{}", self.vars.get(BACKGROUND_IMAGE)))
                    .size(renderer.safe_width as f64, renderer.safe_height as f64)
                    .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                    .create(ui_container),
            )
        } else {
            None
        };
        self.active = true;
        self.background = background;
    }

    fn deinit(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {
        self.background.take();
    }

    fn on_active(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {}

    fn on_deactive(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {}

    fn tick(&mut self, _: f64, renderer: &mut Renderer, ui_container: &mut Container) -> Option<Box<dyn Screen>> {
        let hide = self.screen_sys.is_any_ingame();
        if self.active {
            if hide {
                self.active = false;
                self.deinit(renderer, ui_container);
            }
        } else if !hide {
            self.init(renderer, ui_container);
        }
        None
    }

    fn on_resize(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        self.deinit(renderer, ui_container);
        self.init(renderer, ui_container);
    }

    fn is_tick_always(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}