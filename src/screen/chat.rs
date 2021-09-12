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

use std::cell::Cell;
use std::rc::Rc;
use std::sync::{mpsc, Arc};
use std::thread;

use rand::{self, Rng};

use crate::auth;
use crate::console;
use crate::protocol;
use crate::protocol::mojang;
use crate::render;
use crate::settings;
use crate::ui;
use parking_lot::RwLock;
use leafish_protocol::format::Component;
use crate::ui::{FormattedRef, VAttach, HAttach, ImageRef, TextBuilder, TextRef, Container};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{UNIX_EPOCH, SystemTime};
use instant::Duration;
use crate::render::hud::Hud;
use core::cmp;
use crate::render::{hud, Renderer};
use crate::screen::Screen;

pub const MAX_MESSAGES: usize = 200;
pub const START_TICKS: usize = 5 * 20;
pub const FADE_OUT_START_TICKS: usize = 1 * 20;

pub struct ChatContext {
    messages: Arc<RwLock<Vec<(usize, Component)>>>,
    dirty: AtomicBool,
}

impl ChatContext {

    pub fn new() -> Self {
        ChatContext {
            messages: Arc::new(Default::default()),
            dirty: Default::default(),
        }
    }

    pub fn push_msg(&self, message: Component) {
        if self.messages.clone().read().len() >= MAX_MESSAGES {
            self.messages.clone().write().remove(0);
        }
        self.messages.clone().write().push((START_TICKS, message));
        self.dirty.store(true, Ordering::Release);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    pub fn tick_visible_messages(&self) -> Vec<(usize, Component)> { // TODO: Provide all non-faded out messages and decrement their life counter
        let mut ret = vec![];
        for message in self.messages.clone().write().iter_mut().rev() {
            if message.0 == 0 {
                break;
            }
            message.0 -= 1;
            if message.0 > 0 {
                ret.push((message.0, message.1.clone()));
            }
        }
        ret
    }

}

#[derive(Clone)]
pub struct Chat {
    rendered_messages: Vec<FormattedRef>,
    background: Vec<ImageRef>,
    animated_tex: Option<TextRef>,
    context: Arc<ChatContext>,
    written: String,
    animation: u8,
    offset: f64, // TODO: Implement this!
}

impl Chat {
    pub fn new(context: Arc<ChatContext>) -> Self {
        Chat {
            rendered_messages: vec![],
            background: vec![],
            animated_tex: None,
            context,
            written: String::new(),
            animation: 0,
            offset: 0.0,
        }
    }
}

impl super::Screen for Chat {
    fn on_active(&mut self, renderer: &mut render::Renderer, ui_container: &mut ui::Container) {
        let scale = Hud::icon_scale(renderer);
        let history_size = self.context.messages.clone().read().len();

        let mut component_lines = 0;
        for i in 0..cmp::min(10, history_size) {
            let message =
                self.context.messages.clone().read()[history_size - 1 - i].clone();
            let lines = (renderer.ui.size_of_string(&*message.1.to_string()) / (hud::CHAT_WIDTH * scale))
                .ceil() as u8;
            component_lines += lines;
        }

        if history_size > 0 {
            self.background.push(
                ui::ImageBuilder::new()
                    .draw_index(0)
                    .texture("leafish:solid")
                    .alignment(VAttach::Bottom, HAttach::Left)
                    .position(1.0 * scale, scale * 85.0 / 2.0)
                    .size(
                        500.0 / 2.0 * scale,
                        5.0 * scale * (component_lines as f64)
                            + cmp::min(10, history_size) as f64 * 0.4 * scale,
                    )
                    .colour((0, 0, 0, 100))
                    .create(ui_container)
            );
        }
        self.background.push(
            ui::ImageBuilder::new()
                .draw_index(0)
                .texture("leafish:solid")
                .alignment(VAttach::Bottom, HAttach::Left)
                .position(1.0 * scale, 1.0 * scale)
                .size(
                    renderer.safe_width as f64 - 2.0 * scale,
                    (5.0 * scale
                        + 0.4 * scale) * 1.5,
                )
                .colour((0, 0, 0, 100))
                .create(ui_container)
        );

        let mut component_lines = 0;
        for i in 0..cmp::min(10, history_size) {
            let message =
                self.context.messages.clone().read()[history_size - 1 - i].clone();
            let lines = (renderer.ui.size_of_string(&*message.1.to_string()) / (hud::CHAT_WIDTH * scale))
                .ceil() as u8;
            let text = ui::FormattedBuilder::new()
                .draw_index(0)
                .alignment(VAttach::Bottom, HAttach::Left)
                .position(
                    1.0 * scale,
                    scale * 85.0 / 2.0
                        + ((component_lines as f64) * 5.0) * scale
                        + i as f64 * 0.4 * scale,
                )
                .text(message.1)
                .max_width(hud::CHAT_WIDTH * scale)
                .create(ui_container);
            self.rendered_messages.push(text);
            component_lines += lines;
        }
    }

    fn on_deactive(&mut self, _renderer: &mut render::Renderer, _ui_container: &mut ui::Container) {
        self.rendered_messages.clear();
        self.background.clear();
        self.animated_tex = None;
    }

    fn tick(
        &mut self,
        _delta: f64,
        renderer: &mut render::Renderer,
        ui_container: &mut ui::Container,
    ) -> Option<Box<dyn super::Screen>> {
        let scale = Hud::icon_scale(renderer);
        if self.animation == 0 {
            self.animation = 20;
            self.animated_tex = Some(TextBuilder::new()
                .text("_")
                .alignment(VAttach::Bottom, HAttach::Left)
                .position(renderer.ui.size_of_string(&*self.written) + 2.0 * scale, 2.0 * scale)
                .create(ui_container));
        } else {
            if self.animation == 10 {
                self.animated_tex = None;
            }
            self.animation -= 1;
        }
        if self.context.dirty.load(Ordering::Acquire) {
            self.context.dirty.store(false, Ordering::Release);
            self.rendered_messages.clear();
            self.render_chat(renderer, ui_container);
        }
        None
    }

    fn is_closable(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}

impl Chat {

    fn render_chat(&mut self, renderer: &Renderer, ui_container: &mut Container) {
        let scale = Hud::icon_scale(renderer);
        let history_size = self.context.messages.clone().read().len();

        let mut component_lines = 0;
        for i in 0..cmp::min(10, history_size) {
            let message =
                self.context.messages.clone().read()[history_size - 1 - i].clone();
            let lines = (renderer.ui.size_of_string(&*message.1.to_string()) / (hud::CHAT_WIDTH * scale))
                .ceil() as u8;
            component_lines += lines;
        }

        if history_size > 0 {
            self.background.push(
                ui::ImageBuilder::new()
                    .draw_index(0)
                    .texture("leafish:solid")
                    .alignment(VAttach::Bottom, HAttach::Left)
                    .position(1.0 * scale, scale * 85.0 / 2.0)
                    .size(
                        500.0 / 2.0 * scale,
                        5.0 * scale * (component_lines as f64)
                            + cmp::min(10, history_size) as f64 * 0.4 * scale,
                    )
                    .colour((0, 0, 0, 100))
                    .create(ui_container)
            );
        }
        self.background.push(
            ui::ImageBuilder::new()
                .draw_index(0)
                .texture("leafish:solid")
                .alignment(VAttach::Bottom, HAttach::Left)
                .position(1.0 * scale, 1.0 * scale)
                .size(
                    renderer.safe_width as f64 - 2.0 * scale,
                    (5.0 * scale
                        + 0.4 * scale) * 1.5,
                )
                .colour((0, 0, 0, 100))
                .create(ui_container)
        );

        let mut component_lines = 0;
        for i in 0..cmp::min(10, history_size) {
            let message =
                self.context.messages.clone().read()[history_size - 1 - i].clone();
            let lines = (renderer.ui.size_of_string(&*message.1.to_string()) / (hud::CHAT_WIDTH * scale))
                .ceil() as u8;
            let text = ui::FormattedBuilder::new()
                .draw_index(0)
                .alignment(VAttach::Bottom, HAttach::Left)
                .position(
                    1.0 * scale,
                    scale * 85.0 / 2.0
                        + ((component_lines as f64) * 5.0) * scale
                        + i as f64 * 0.4 * scale,
                )
                .text(message.1)
                .max_width(hud::CHAT_WIDTH * scale)
                .create(ui_container);
            self.rendered_messages.push(text);
            component_lines += lines;
        }

        /*for message in self.context.messages.clone().write().iter_mut().rev().enumerate() {
            if message.1.0 != 0 { // TODO: Should we use size - 1 instead of 0 here?
                message.1.0 -= 1;
            } else {
                if message.0 == 0 {
                    self.context.dirty.store(false, Ordering::Release);
                }
            }
        }*/
    }

}
