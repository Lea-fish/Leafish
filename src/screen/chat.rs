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

use std::sync::Arc;

use crate::protocol::packet;
use crate::render::hud::{Hud, START_TICKS};
use crate::render::{hud, Renderer};
use crate::screen::{Screen, ScreenSystem, ScreenType};
use crate::ui::{Container, FormattedRef, HAttach, ImageRef, TextBuilder, TextRef, VAttach};
use crate::Game;
use crate::{ui, KeyCmp};
use core::cmp;
use leafish_protocol::format::Component;
use parking_lot::RwLock;
use shared::Version;
use std::sync::atomic::{AtomicBool, Ordering};
use winit::keyboard::{Key, NamedKey, PhysicalKey};

pub const MAX_MESSAGES: usize = 200;
const MAX_MESSAGE_LENGTH_PRE_1_11: usize = 100;
const MAX_MESSAGE_LENGTH_SINCE_1_11: usize = 256;

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
        if self.messages.read().len() >= MAX_MESSAGES {
            self.messages.write().remove(0);
        }
        self.messages.write().push((START_TICKS, message));
        self.dirty.store(true, Ordering::Release);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    pub fn tick_visible_messages(&self) -> Vec<(usize, Component)> {
        // TODO: Provide all non-faded out messages and decrement their life counter
        let mut ret = vec![];
        for message in self.messages.write().iter_mut().rev() {
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

impl Default for ChatContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Chat {
    rendered_messages: Vec<FormattedRef>,
    background: Vec<ImageRef>,
    animated_tex: Option<TextRef>,
    written_text: Option<TextRef>,
    context: Arc<ChatContext>,
    written: String,
    animation: u8,
    #[allow(dead_code)]
    offset: f64, // TODO: Implement this (scrolling in chat)!
    dirty_written: bool,
}

impl Chat {
    pub fn new(context: Arc<ChatContext>) -> Self {
        Chat {
            rendered_messages: vec![],
            background: vec![],
            animated_tex: None,
            written_text: None,
            context,
            written: String::new(),
            animation: 0,
            offset: 0.0,
            dirty_written: false,
        }
    }
}

impl super::Screen for Chat {
    fn on_active(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<Renderer>,
        ui_container: &mut ui::Container,
    ) {
        /*let scale = Hud::icon_scale(renderer);
        let history_size = self.context.messages.clone().read().len();

        let mut component_lines = 0;
        for i in 0..cmp::min(10, history_size) {
            let message = self.context.messages.clone().read()[history_size - 1 - i].clone();
            let lines = (renderer.ui.size_of_string(&*message.1.to_string())
                / hud::CHAT_WIDTH)
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
                        (5.0 * (component_lines as f64)
                            + cmp::min(10, history_size) as f64 * 0.4) * scale,
                    )
                    .colour((0, 0, 0, 100))
                    .create(ui_container),
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
                    (5.0 + 0.4) * 1.5 * scale,
                )
                .colour((0, 0, 0, 100))
                .create(ui_container),
        );

        let mut component_lines = 0;
        for i in 0..cmp::min(10, history_size) {
            let message = self.context.messages.clone().read()[history_size - 1 - i].clone();
            let lines = (renderer.ui.size_of_string(&*message.1.to_string())
                / hud::CHAT_WIDTH)
                .ceil() as u8;
            let text = ui::FormattedBuilder::new()
                .draw_index(0)
                .alignment(VAttach::Bottom, HAttach::Left)
                .position(
                    1.0 * scale,
                    (85.0 / 2.0
                        + ((component_lines as f64) * 5.0)
                        + i as f64 * 0.4) * scale,
                )
                .text(message.1)
                .max_width(hud::CHAT_WIDTH * scale)
                .create(ui_container);
            self.rendered_messages.push(text);
            component_lines += lines;
        }*/
        self.render_chat(renderer, ui_container);
    }

    fn on_deactive(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<Renderer>,
        _ui_container: &mut ui::Container,
    ) {
        self.rendered_messages.clear();
        self.background.clear();
        self.animated_tex = None;
    }

    fn tick(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<Renderer>,
        ui_container: &mut ui::Container,
        _delta: f64,
    ) {
        let scale = Hud::icon_scale(&renderer);
        if self.animation == 0 {
            self.animation = 20;
            self.animated_tex = Some(
                TextBuilder::new()
                    .text("_")
                    .alignment(VAttach::Bottom, HAttach::Left)
                    .position(
                        renderer.ui.lock().size_of_string(&self.written) + 2.0 * scale,
                        2.0 * scale,
                    )
                    .create(ui_container),
            );
        } else {
            if self.animation == 10 {
                self.animated_tex = None;
            }
            self.animation -= 1;
        }
        if self.dirty_written {
            self.dirty_written = false;
            if self.animated_tex.is_some() {
                self.animated_tex = Some(
                    TextBuilder::new()
                        .text("_")
                        .alignment(VAttach::Bottom, HAttach::Left)
                        .position(
                            renderer.ui.lock().size_of_string(&self.written) + 2.0 * scale,
                            2.0 * scale,
                        )
                        .create(ui_container),
                );
            }
            self.written_text = Some(
                TextBuilder::new()
                    .text(self.written.clone())
                    .alignment(VAttach::Bottom, HAttach::Left)
                    .position(2.0 * scale, 2.0 * scale)
                    .create(ui_container),
            );
        }
        if self.context.dirty.load(Ordering::Acquire) {
            self.context.dirty.store(false, Ordering::Release);
            self.rendered_messages.clear();
            self.background.clear();
            self.render_chat(renderer, ui_container);
        }
    }

    fn on_resize(
        &mut self,
        screen_sys: &ScreenSystem,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        self.on_deactive(screen_sys, renderer.clone(), ui_container);
        self.on_active(screen_sys, renderer, ui_container);
    }

    fn on_key_press(&mut self, key: (Key, PhysicalKey), down: bool, game: &mut Game) {
        if key.0 == Key::Named(NamedKey::Escape) && !down {
            game.screen_sys.pop_screen();
            return;
        }
        if key.0 == Key::Named(NamedKey::Enter) && !down {
            if !self.written.is_empty() {
                game.server.as_ref().unwrap().write_packet(
                    packet::play::serverbound::ChatMessage {
                        message: self.written.clone(),
                    },
                );
            }
            game.screen_sys.pop_screen();
            return;
        }
        if key.0.eq_ignore_case('v') && game.is_ctrl_pressed {
            if let Ok(clipboard) = game.clipboard_provider.lock().get_contents() {
                for c in clipboard.chars() {
                    if self.written.len()
                        >= if game.server.as_ref().unwrap().mapped_protocol_version
                            >= Version::V1_11
                        {
                            MAX_MESSAGE_LENGTH_SINCE_1_11
                        } else {
                            MAX_MESSAGE_LENGTH_PRE_1_11
                        }
                    {
                        break;
                    }
                    self.written.push(c);
                }
                self.dirty_written = true;
            }
            return;
        }
        if key.0 == Key::Named(NamedKey::Backspace) {
            // Handle backspace
            if !self.written.is_empty() {
                self.written.pop();
                self.dirty_written = true;
            }
            return;
        }
        if let Some(str) = key.0.to_text() {
            if str.len() != 1 {
                panic!("weird input!");
            }
            const ILLEGAL_CHARS: &[char] =
                &[13 as char, 127 as char, 167 as char, '§', 255 as char];
            let curr = str.chars().next().unwrap();
            if !ILLEGAL_CHARS.iter().any(|illegal| curr == *illegal) {
                if self.written.len()
                    >= if game.server.as_ref().unwrap().mapped_protocol_version >= Version::V1_11 {
                        MAX_MESSAGE_LENGTH_SINCE_1_11
                    } else {
                        MAX_MESSAGE_LENGTH_PRE_1_11
                    }
                {
                    return;
                }
                self.written.push_str(str);
                self.dirty_written = true;
            }
        }
    }

    fn is_closable(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }

    fn ty(&self) -> ScreenType {
        ScreenType::Chat
    }
}

impl Chat {
    fn render_chat(&mut self, renderer: Arc<Renderer>, ui_container: &mut Container) {
        let scale = Hud::icon_scale(&renderer);
        let history_size = self.context.messages.read().len();

        let mut component_lines = 0;
        for i in 0..cmp::min(10, history_size) {
            let message = self.context.messages.read()[history_size - 1 - i].clone();
            let lines = (renderer.ui.lock().size_of_string(&message.1.to_string())
                / (hud::CHAT_WIDTH * scale))
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
                    .create(ui_container),
            );
        }
        self.background.push(
            ui::ImageBuilder::new()
                .draw_index(0)
                .texture("leafish:solid")
                .alignment(VAttach::Bottom, HAttach::Left)
                .position(1.0 * scale, 1.0 * scale)
                .size(
                    renderer.screen_data.read().safe_width as f64 - 2.0 * scale,
                    (5.0 * scale + 0.4 * scale) * 1.5,
                )
                .colour((0, 0, 0, 100))
                .create(ui_container),
        );

        let mut component_lines = 0;
        for i in 0..cmp::min(10, history_size) {
            let message = self.context.messages.read()[history_size - 1 - i].clone();
            let lines = (renderer.ui.lock().size_of_string(&message.1.to_string())
                / (hud::CHAT_WIDTH * scale))
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
}
