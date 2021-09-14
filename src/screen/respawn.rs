use crate::render::hud::Hud;
use crate::render::Renderer;
use crate::screen::Screen;
use crate::ui;
use crate::ui::{Container, ImageRef};
use leafish_protocol::protocol::packet::play::serverbound::ClientStatus;
use leafish_protocol::protocol::{VarInt, Version};

pub struct Respawn {
    elements: Option<UIElements>,
    score: u32,
}

impl Clone for Respawn {
    fn clone(&self) -> Self {
        Respawn {
            elements: None,
            score: self.score,
        }
    }
}

struct UIElements {
    _background: ImageRef,

    _text: ui::TextRef,
    _score_text: ui::TextRef,
    _respawn_button: ui::ButtonRef,
    _main_screen_button: ui::ButtonRef,
}

impl Respawn {
    pub fn new(score: u32) -> Self {
        Respawn {
            elements: None,
            score,
        }
    }
}

impl super::Screen for Respawn {
    fn on_active(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        let icon_scale = Hud::icon_scale(renderer);
        let background = ui::ImageBuilder::new()
            .texture("leafish:solid")
            .position(0.0, 0.0)
            .size(renderer.width as f64, renderer.height as f64)
            .colour((104, 0, 0, 100))
            .create(ui_container);
        let text = ui::TextBuilder::new()
            .text("You died!")
            .position(0.0, -(icon_scale * 10.0 * 3.0))
            .colour((255, 255, 255, 255))
            .scale_y(icon_scale)
            .scale_x(icon_scale)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        let score_text = ui::TextBuilder::new()
            .text(format!("Score: {}", self.score)) // TODO: Make the score yellow!
            .position(0.0, -(icon_scale * 5.0 * 3.0))
            .colour((255, 255, 255, 255))
            .scale_y(icon_scale / 3.0)
            .scale_x(icon_scale / 3.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        let respawn_button = ui::ButtonBuilder::new()
            .position(0.0, 0.0)
            .size(icon_scale * 20.0 * 3.0, icon_scale * 4.0 * 3.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut respawn_button = respawn_button.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Respawn")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *respawn_button);
            respawn_button.add_text(txt);
            respawn_button.add_click_func(|_, game| {
                let server = game.server.as_ref().unwrap().clone();

                // TODO: Use ClientStatus_u8 instead!
                #[allow(clippy::if_same_then_else)]
                let packet = if Version::V1_8 > Version::from_id(server.protocol_version as u32) {
                    ClientStatus {
                        action_id: VarInt(0),
                    }
                } else {
                    ClientStatus {
                        action_id: VarInt(0),
                    }
                };
                game.server.as_ref().unwrap().clone().write_packet(packet);
                true
            });
        }
        let main_menu_button = ui::ButtonBuilder::new()
            .position(0.0, icon_scale * 10.0 * 3.0)
            .size(icon_scale * 20.0 * 3.0, icon_scale * 4.0 * 3.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut main_menu_button = main_menu_button.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Title screen")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *main_menu_button);
            main_menu_button.add_text(txt);
            main_menu_button.add_click_func(|_, _game| {
                // TODO: Disconnect!
                true
            });
        }
        self.elements = Some(UIElements {
            _background: background,
            _text: text,
            _score_text: score_text,
            _respawn_button: respawn_button,
            _main_screen_button: main_menu_button,
        });
    }

    fn on_deactive(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {
        self.elements = None;
    }

    fn tick(
        &mut self,
        _delta: f64,
        _renderer: &mut Renderer,
        _ui_container: &mut Container,
    ) -> Option<Box<dyn Screen>> {
        // TODO
        None
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}
