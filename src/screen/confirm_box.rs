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

use crate::render;
use crate::ui;
use crate::Game;

use crate::screen::Screen;
use std::rc::Rc;

pub struct ConfirmBox {
    elements: Option<UIElements>,
    prompt: String,
    cancel_callback: Rc<dyn Fn(&mut Game)>,
    confirm_callback: Rc<dyn Fn(&mut Game)>,
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
    logo: ui::logo::Logo,

    _prompt: ui::TextRef,
    _confirm: ui::ButtonRef,
    _cancel: ui::ButtonRef,
}

impl ConfirmBox {
    pub fn new(
        prompt: String,
        cancel_callback: Rc<dyn Fn(&mut Game)>,
        confirm_callback: Rc<dyn Fn(&mut Game)>,
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
    fn on_active(&mut self, renderer: &mut render::Renderer, ui_container: &mut ui::Container) {
        let logo = ui::logo::Logo::new(renderer.resources.clone(), ui_container);

        // Prompt
        let prompt = ui::TextBuilder::new()
            .text(
                /*format!(
                    "Are you sure you wish to delete {} {}?",
                    self.name, self.address
                )*/
                self.prompt.clone(),
            )
            .position(0.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);

        // Confirm
        let confirm = ui::ButtonBuilder::new()
            .position(110.0, 100.0)
            .size(200.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut confirm = confirm.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Confirm")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *confirm);
            confirm.add_text(txt);
            let callback = self.confirm_callback.clone();
            confirm.add_click_func(move |_, game| {
                (*callback)(game);
                true
            });
        }

        // Cancel
        let cancel = ui::ButtonBuilder::new()
            .position(-110.0, 100.0)
            .size(200.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        {
            let mut cancel = cancel.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Cancel")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
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

    fn on_deactive(&mut self, _renderer: &mut render::Renderer, _ui_container: &mut ui::Container) {
        // Clean up
        self.elements = None
    }

    fn tick(
        &mut self,
        _delta: f64,
        renderer: &mut render::Renderer,
        _ui_container: &mut ui::Container,
    ) -> Option<Box<dyn super::Screen>> {
        let elements = self.elements.as_mut().unwrap();
        elements.logo.tick(renderer);
        None
    }

    fn is_closable(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}
