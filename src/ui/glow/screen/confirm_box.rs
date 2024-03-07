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

use crate::ui::glow::render::Renderer;
use crate::ui::glow::ui::logo::Logo;
use crate::ui::glow::ui::{ButtonBuilder, ButtonRef, Container, HAttach, TextBuilder, TextRef, VAttach};
use crate::Game;

use std::rc::Rc;
use std::sync::Arc;

use super::{Screen, ScreenSystem};

pub struct ConfirmBox {
    elements: Option<UIElements>,
    prompt: String,
    cancel_callback: Rc<dyn Fn(&Arc<Game>)>,
    confirm_callback: Rc<dyn Fn(&Arc<Game>)>,
}

impl Clone for ConfirmBox {
    fn clone(&self) -> Self {
        Self {
            elements: None,
            prompt: self.prompt.clone(),
            cancel_callback: self.cancel_callback.clone(),
            confirm_callback: self.confirm_callback.clone(),
        }
    }
}

struct UIElements {
    logo: Logo,

    _prompt: TextRef,
    _confirm: ButtonRef,
    _cancel: ButtonRef,
}

impl ConfirmBox {
    pub fn new(
        prompt: String,
        cancel_callback: Rc<dyn Fn(&Arc<Game>)>,
        confirm_callback: Rc<dyn Fn(&Arc<Game>)>,
    ) -> Self {
        Self {
            elements: None,
            prompt,
            cancel_callback,
            confirm_callback,
        }
    }
}

impl super::Screen for ConfirmBox {
    fn on_active(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        let logo = Logo::new(renderer.resources.clone(), ui_container);

        // Prompt
        let prompt = TextBuilder::new()
            .text(self.prompt.clone())
            .position(0.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);

        // Confirm
        let confirm = ButtonBuilder::new()
            .position(110.0, 100.0)
            .size(200.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut confirm = confirm.borrow_mut();
            let txt = TextBuilder::new()
                .text("Confirm")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *confirm);
            confirm.add_text(txt);
            let callback = self.confirm_callback.clone();
            confirm.add_click_func(move |_, game| {
                (*callback)(game);
                true
            });
        }

        // Cancel
        let cancel = ButtonBuilder::new()
            .position(-110.0, 100.0)
            .size(200.0, 40.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .create(ui_container);
        {
            let mut cancel = cancel.borrow_mut();
            let txt = TextBuilder::new()
                .text("Cancel")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *cancel);
            cancel.add_text(txt);
            let callback = self.cancel_callback.clone();
            cancel.add_click_func(move |_, game| {
                (*callback)(game);
                true
            });
        }

        self.elements = Some(UIElements {
            logo,
            _prompt: prompt,
            _confirm: confirm,
            _cancel: cancel,
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

    fn is_closable(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}
