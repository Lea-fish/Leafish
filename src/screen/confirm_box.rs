// Copyright 2016 Matthew Collins
// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::render;
use crate::ui;
use crate::Game;

use crate::screen::{Screen, ScreenSystem};
use std::rc::Rc;
use std::sync::Arc;

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
    fn on_active(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<render::Renderer>,
        ui_container: &mut ui::Container,
    ) {
        let logo = ui::logo::Logo::new(renderer.resources.clone(), ui_container);

        // Prompt
        let prompt = ui::TextBuilder::new()
            .text(self.prompt.clone())
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

    fn on_deactive(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<render::Renderer>,
        _ui_container: &mut ui::Container,
    ) {
        // Clean up
        self.elements = None
    }

    fn tick(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<render::Renderer>,
        _ui_container: &mut ui::Container,
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
