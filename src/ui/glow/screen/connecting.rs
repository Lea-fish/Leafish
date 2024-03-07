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

use crate::ui::glow::{render::Renderer, ui::{logo::Logo, Container, HAttach, TextBuilder, TextRef, VAttach}};
use std::sync::Arc;

use super::{Screen, ScreenSystem};

pub struct Connecting {
    elements: Option<UIElements>,
    target: String,
}

impl Clone for Connecting {
    fn clone(&self) -> Self {
        Connecting {
            elements: None,
            target: self.target.clone(),
        }
    }
}

struct UIElements {
    logo: Logo,
    _connect_msg: TextRef,
    _msg: TextRef,
    _disclaimer: TextRef,
}

impl Connecting {
    pub fn new(target: &str) -> Connecting {
        Connecting {
            elements: None,
            target: target.to_owned(),
        }
    }
}

impl super::Screen for Connecting {
    fn on_active(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        let logo = Logo::new(renderer.resources.clone(), ui_container);

        let connect_msg = TextBuilder::new()
            .text("Connecting to")
            .position(0.0, -16.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);

        let msg = TextBuilder::new()
            .text(self.target.clone())
            .position(0.0, 16.0)
            .colour((255, 255, 85, 255))
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);

        // Disclaimer
        let disclaimer = TextBuilder::new()
            .text("Not affiliated with Mojang/Minecraft")
            .position(5.0, 5.0)
            .colour((255, 200, 200, 255))
            .alignment(VAttach::Bottom, HAttach::Right)
            .create(ui_container);

        self.elements = Some(UIElements {
            logo,
            _disclaimer: disclaimer,
            _msg: msg,
            _connect_msg: connect_msg,
        });
    }
    fn on_deactive(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        _renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
        // Clean up
        self.elements = None
    }

    fn tick(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
        _delta: f64,
    ) {
        let elements = self.elements.as_mut().unwrap();
        elements.logo.tick(renderer);
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}
