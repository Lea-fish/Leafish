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

use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

use crate::format;
use crate::format::{Component, ComponentType};
use crate::paths;
use crate::protocol;
use crate::ui::glow::ctx;
use crate::ui::glow::render::hud::{Hud, HudContext};
use crate::ui::glow::render::Renderer;
use crate::ui::glow::ui::logo::Logo;
use crate::ui::glow::ui::{ButtonBuilder, ButtonRef, Container, Formatted, FormattedBuilder, FormattedRef, HAttach, ImageBuilder, ImageRef, TextBuilder, TextRef, VAttach};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use crossbeam_channel::unbounded;
use crossbeam_channel::{Receiver, TryRecvError};
use instant::Duration;
use parking_lot::RwLock;
use rand::Rng;
use serde_json::Value;
use std::collections::BTreeMap;

use super::{Screen, ScreenSystem};

pub struct ServerList {
    elements: Option<UIElements>,
    disconnect_reason: Option<Component>,

    needs_reload: Rc<RefCell<bool>>,
}

impl Clone for ServerList {
    fn clone(&self) -> Self {
        ServerList {
            elements: None,
            disconnect_reason: self.disconnect_reason.clone(),
            needs_reload: Rc::new(RefCell::new(false)),
        }
    }
}

struct UIElements {
    logo: Logo,
    servers: Vec<Server>,

    _add_btn: ButtonRef,
    _refresh_btn: ButtonRef,
    _options_btn: ButtonRef,
    _disclaimer: TextRef,

    _disconnected: Option<ImageRef>,
}

struct Server {
    back: ImageRef,
    offset: f64,
    y: f64,

    motd: FormattedRef,
    ping: ImageRef,
    players: TextRef,
    version: FormattedRef,

    icon: ImageRef,
    icon_texture: Option<String>,

    done_ping: bool,
    recv: Receiver<PingInfo>,
}

struct PingInfo {
    motd: format::Component,
    ping: Duration,
    exists: bool,
    online: i32,
    max: i32,
    protocol_version: i32,
    protocol_name: String,
    forge_mods: Vec<crate::protocol::forge::ForgeMod>,
    favicon: Option<image::DynamicImage>,
}

impl Server {
    fn update_position(&mut self) {
        if self.offset < 0.0 {
            self.y = self.offset * 200.0;
        } else {
            self.y = self.offset * 100.0;
        }
    }
}

impl ServerList {
    pub fn new(disconnect_reason: Option<Component>) -> ServerList {
        ServerList {
            elements: None,
            disconnect_reason,
            needs_reload: Rc::new(RefCell::new(false)),
        }
    }

    fn reload_server_list(
        &mut self,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        let elements = self.elements.as_mut().unwrap();
        *self.needs_reload.borrow_mut() = false;
        {
            // Clean up previous list icons.
            let mut tex = renderer.get_textures_ref().write();
            for server in &mut elements.servers {
                if let Some(ref icon) = server.icon_texture {
                    tex.remove_dynamic(icon);
                }
            }
        }
        elements.servers.clear();

        let file = match fs::File::open(paths::get_data_dir().join("servers.json")) {
            Ok(val) => val,
            Err(_) => return,
        };
        let servers_info: serde_json::Value = serde_json::from_reader(file).unwrap();
        let servers = servers_info.get("servers").unwrap().as_array().unwrap();
        let mut offset = 0.0;

        for (index, svr) in servers.iter().enumerate() {
            let name = svr.get("name").unwrap().as_str().unwrap().to_owned();
            let address = svr.get("address").unwrap().as_str().unwrap().to_owned();

            // Everything is attached to this
            let back = ImageBuilder::new()
                .texture("leafish:solid")
                .position(0.0, offset * 100.0)
                .size(700.0, 100.0)
                .colour((0, 0, 0, 100))
                .alignment(VAttach::Middle, HAttach::Center)
                .create(ui_container);

            let (send, recv) = unbounded();
            // Make whole entry interactable
            {
                let mut backr = back.borrow_mut();
                let address = address.clone();
                backr.add_hover_func(move |this, over, _| {
                    this.colour.3 = if over { 200 } else { 100 };
                    false
                });
                backr.add_click_func(move |_, game| {
                    let ctx = ctx();
                    ctx.screen_sys
                        .clone()
                        .replace_screen(Box::new(super::connecting::Connecting::new(&address)));
                    let hud_context = Arc::new(RwLock::new(HudContext::new()));
                    let result = game.connect_to(&address);
                    ctx.screen_sys.clone().pop_screen();
                    if let Err(error) = result {
                        ctx.screen_sys
                            .clone()
                            .add_screen(Box::new(ServerList::new(Some(Component::new(
                                ComponentType::new(&error.to_string(), None),
                            )))));
                    } else {
                        ctx.screen_sys
                            .clone()
                            .add_screen(Box::new(Hud::new(hud_context)));
                    }
                    true
                });
            }

            // Server name
            TextBuilder::new()
                .text(name.clone())
                .position(100.0, 5.0)
                .attach(&mut *back.borrow_mut());

            // Server icon
            let icon = ImageBuilder::new()
                .texture("misc/unknown_server")
                .position(5.0, 5.0)
                .size(90.0, 90.0)
                .attach(&mut *back.borrow_mut());

            // Ping indicator
            let ping = ImageBuilder::new()
                .texture("gui/icons")
                .position(5.0, 5.0)
                .size(20.0, 16.0)
                .texture_coords((0.0, 56.0, 10.0, 8.0))
                .alignment(VAttach::Top, HAttach::Right)
                .attach(&mut *back.borrow_mut());

            // Player count
            let players = TextBuilder::new()
                .text("???")
                .position(30.0, 5.0)
                .alignment(VAttach::Top, HAttach::Right)
                .attach(&mut *back.borrow_mut());

            // Server's message of the day
            let motd = FormattedBuilder::new()
                .text(Component::new(ComponentType::new("Connecting...", None)))
                .position(100.0, 23.0)
                .max_width(700.0 - (90.0 + 10.0 + 5.0))
                .attach(&mut *back.borrow_mut());

            // Version information
            let version = FormattedBuilder::new()
                .text(Component::new(ComponentType::new("", None)))
                .position(100.0, 5.0)
                .max_width(700.0 - (90.0 + 10.0 + 5.0))
                .alignment(VAttach::Bottom, HAttach::Left)
                .attach(&mut *back.borrow_mut());

            // Delete entry button
            let delete_entry = ButtonBuilder::new()
                .position(0.0, 0.0)
                .size(25.0, 25.0)
                .alignment(VAttach::Bottom, HAttach::Right)
                .attach(&mut *back.borrow_mut());
            {
                let mut btn = delete_entry.borrow_mut();
                let txt = TextBuilder::new()
                    .text("X")
                    .alignment(VAttach::Middle, HAttach::Center)
                    .attach(&mut *btn);
                btn.add_text(txt);
                let sname = name.clone();
                let saddr = address.clone();
                btn.add_click_func(move |_, _game| {
                    let text = format!("Are you sure you wish to delete {} {}?", &sname, &saddr);
                    ctx().screen_sys.add_screen(Box::new(
                        super::confirm_box::ConfirmBox::new(
                            text,
                            Rc::new(|_game| {
                                ctx().screen_sys.pop_screen();
                            }),
                            Rc::new(move |_game| {
                                ctx().screen_sys.pop_screen();
                                Self::delete_server(index);
                            }),
                        ),
                    ));
                    true
                })
            }

            // Edit entry button
            let edit_entry = ButtonBuilder::new()
                .position(25.0, 0.0)
                .size(25.0, 25.0)
                .alignment(VAttach::Bottom, HAttach::Right)
                .attach(&mut *back.borrow_mut());
            {
                let mut btn = edit_entry.borrow_mut();
                let txt = TextBuilder::new()
                    .text("E")
                    .alignment(VAttach::Middle, HAttach::Center)
                    .attach(&mut *btn);
                btn.add_text(txt);
                let sname = name.clone();
                let saddr = address.clone();
                btn.add_click_func(move |_, _game| {
                    ctx().screen_sys.replace_screen(Box::new(
                        super::edit_server::EditServerEntry::new(Some((
                            index,
                            sname.clone(),
                            saddr.clone(),
                        ))),
                    ));
                    true
                })
            }

            let mut server = Server {
                back,
                offset,
                y: 0.0,
                done_ping: false,
                recv,

                motd,
                ping,
                players,
                version,

                icon,
                icon_texture: None,
            };
            server.update_position();
            elements.servers.push(server);
            offset += 1.0;

            // Don't block the main thread whilst pinging the server
            thread::spawn(move || {
                match protocol::Conn::new(&address, protocol::SUPPORTED_PROTOCOLS[0])
                    .and_then(|conn| conn.do_status())
                {
                    Ok(res) => {
                        let desc = res.0.description;
                        let favicon = if let Some(icon) = res.0.favicon {
                            let data_base64 = &icon["data:image/png;base64,".len()..];
                            let data_base64: String =
                                data_base64.chars().filter(|c| !c.is_whitespace()).collect();
                            let data = STANDARD.decode(data_base64).unwrap();
                            Some(image::load_from_memory(&data).unwrap())
                        } else {
                            None
                        };
                        drop(send.send(PingInfo {
                            motd: desc,
                            ping: res.1,
                            exists: true,
                            online: res.0.players.online,
                            max: res.0.players.max,
                            protocol_version: res.0.version.protocol,
                            protocol_name: res.0.version.name,
                            forge_mods: res.0.forge_mods,
                            favicon,
                        }));
                    }
                    Err(err) => {
                        let e = format!("{}", err);
                        let msg = ComponentType::new(&e, Some(format::Color::Red));
                        let _ = send.send(PingInfo {
                            motd: Component::new(msg),
                            ping: Duration::new(99999, 0),
                            exists: false,
                            online: 0,
                            max: 0,
                            protocol_version: 0,
                            protocol_name: "".to_owned(),
                            forge_mods: vec![],
                            favicon: None,
                        });
                    }
                }
            });
        }
    }

    fn delete_server(index: usize) {
        let mut servers_info = match fs::File::open(paths::get_data_dir().join("servers.json")) {
            Ok(val) => serde_json::from_reader(val).unwrap(),
            Err(_) => {
                let mut info = BTreeMap::default();
                info.insert("servers".to_owned(), Value::Array(vec![]));
                Value::Object(info.into_iter().collect())
            }
        };

        {
            let servers = servers_info
                .as_object_mut()
                .unwrap()
                .get_mut("servers")
                .unwrap()
                .as_array_mut()
                .unwrap();
            servers.remove(index);
        }

        let mut out = fs::File::create(paths::get_data_dir().join("servers.json")).unwrap();
        serde_json::to_writer_pretty(&mut out, &servers_info).unwrap();
    }

    fn init_list(&mut self, renderer: &Arc<Renderer>, ui_container: &mut Container) {
        let logo = Logo::new(renderer.resources.clone(), ui_container);

        // Refresh the server list
        let refresh = ButtonBuilder::new()
            .position(300.0, -50.0 - 15.0)
            .size(100.0, 30.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .draw_index(2)
            .create(ui_container);
        {
            let mut refresh = refresh.borrow_mut();
            let txt = TextBuilder::new()
                .text("Refresh")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *refresh);
            refresh.add_text(txt);
            let nr = self.needs_reload.clone();
            refresh.add_click_func(move |_, _| {
                *nr.borrow_mut() = true;
                true
            })
        }

        // Add a new server to the list
        let add = ButtonBuilder::new()
            .position(200.0, -50.0 - 15.0)
            .size(100.0, 30.0)
            .alignment(VAttach::Middle, HAttach::Center)
            .draw_index(2)
            .create(ui_container);
        {
            let mut add = add.borrow_mut();
            let txt = TextBuilder::new()
                .text("Add")
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *add);
            add.add_text(txt);
            add.add_click_func(move |_, _game| {
                ctx().screen_sys
                    .replace_screen(Box::new(super::edit_server::EditServerEntry::new(None)));
                true
            })
        }

        // Options menu
        let options = ButtonBuilder::new()
            .position(5.0, 25.0)
            .size(40.0, 40.0)
            .draw_index(1)
            .alignment(VAttach::Bottom, HAttach::Right)
            .create(ui_container);
        {
            let mut options = options.borrow_mut();
            ImageBuilder::new()
                .texture("leafish:gui/cog")
                .position(0.0, 0.0)
                .size(40.0, 40.0)
                .alignment(VAttach::Middle, HAttach::Center)
                .attach(&mut *options);
            options.add_click_func(|_, game| {
                ctx().screen_sys
                    .add_screen(Box::new(super::SettingsMenu::new(
                        game.settings.clone(),
                        false,
                    )));
                true
            });
        }

        // Disclaimer
        let disclaimer = TextBuilder::new()
            .text("Not affiliated with Mojang/Minecraft")
            .position(5.0, 5.0)
            .colour((255, 200, 200, 255))
            .draw_index(1)
            .alignment(VAttach::Bottom, HAttach::Right)
            .create(ui_container);

        // If we are kicked from a server display the reason
        let disconnected = if let Some(ref disconnect_reason) = self.disconnect_reason {
            let (width, height) = Formatted::compute_size(
                renderer.clone(),
                disconnect_reason,
                600.0,
                1.0,
                1.0,
                1.0,
            );
            let background = ImageBuilder::new()
                .texture("leafish:solid")
                .position(0.0, 3.0)
                .size(
                    width.max(renderer.ui.lock().size_of_string("Disconnected")) + 4.0,
                    height + 4.0 + 16.0,
                )
                .colour((0, 0, 0, 100))
                .alignment(VAttach::Top, HAttach::Center)
                .draw_index(10)
                .create(ui_container);
            TextBuilder::new()
                .text("Disconnected")
                .position(0.0, 2.0)
                .colour((255, 0, 0, 255))
                .alignment(VAttach::Top, HAttach::Center)
                .attach(&mut *background.borrow_mut());
            FormattedBuilder::new()
                .text(disconnect_reason.clone())
                .position(0.0, 18.0)
                .max_width(600.0)
                .alignment(VAttach::Top, HAttach::Center)
                .attach(&mut *background.borrow_mut());
            Some(background)
        } else {
            None
        };

        self.elements = Some(UIElements {
            logo,
            servers: vec![],

            _add_btn: add,
            _refresh_btn: refresh,
            _options_btn: options,
            _disclaimer: disclaimer,

            _disconnected: disconnected,
        });
    }
}

impl super::Screen for ServerList {
    fn on_active(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        self.init_list(renderer, ui_container);
        *self.needs_reload.borrow_mut() = true;
    }

    fn on_deactive(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
        // Clean up
        {
            let elements = self.elements.as_mut().unwrap();
            let mut tex = renderer.get_textures_ref().write();
            for server in &mut elements.servers {
                if let Some(ref icon) = server.icon_texture {
                    tex.remove_dynamic(icon);
                }
            }
        }
        self.elements = None
    }

    fn tick(
        &mut self,
        _screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        delta: f64,
    ) {
        if *self.needs_reload.borrow() {
            self.reload_server_list(renderer, ui_container);
        }
        let elements = self.elements.as_mut().unwrap();

        elements.logo.tick(renderer);

        for s in &mut elements.servers {
            // Animate the entries
            {
                let mut back = s.back.borrow_mut();
                let dy = s.y - back.y;
                if dy * dy > 1.0 {
                    let y = back.y;
                    back.y = y + delta * dy * 0.1;
                } else {
                    back.y = s.y;
                }
            }
            #[allow(clippy::if_same_then_else)]
            if s.y < elements._add_btn.borrow().y {
                // TODO: Make button invisible!
            } else {
                // TODO: Make button visible.
            }

            // Keep checking to see if the server has finished being
            // pinged
            if !s.done_ping {
                match s.recv.try_recv() {
                    Ok(res) => {
                        s.done_ping = true;
                        s.motd.borrow_mut().set_text(res.motd);
                        // Selects the icon for the given ping range
                        // TODO: switch to as_millis() experimental duration_as_u128 #50202 once available?
                        let ping_ms = (res.ping.subsec_nanos() as f64) / 1000000.0
                            + (res.ping.as_secs() as f64) * 1000.0;
                        let y = match ping_ms.round() as u64 {
                            _x @ 0..=75 => 16.0,
                            _x @ 76..=150 => 24.0,
                            _x @ 151..=225 => 32.0,
                            _x @ 226..=350 => 40.0,
                            _x @ 351..=999 => 48.0,
                            _ => 56.0,
                        };
                        s.ping.borrow_mut().texture_coords.1 = y;
                        if res.exists {
                            {
                                let mut players = s.players.borrow_mut();
                                let txt = if protocol::SUPPORTED_PROTOCOLS
                                    .contains(&res.protocol_version)
                                {
                                    players.colour.1 = 255;
                                    players.colour.2 = 255;
                                    format!("{}/{}", res.online, res.max)
                                } else {
                                    players.colour.1 = 85;
                                    players.colour.2 = 85;
                                    format!("Out of date {}/{}", res.online, res.max)
                                };
                                players.text = txt;
                            }
                            let sm =
                                format!("{} mods + {}", res.forge_mods.len(), res.protocol_name);
                            let st = if !res.forge_mods.is_empty() {
                                &sm
                            } else {
                                &res.protocol_name
                            };
                            let msg_component =
                                Component::new(ComponentType::new(st, Some(format::Color::Yellow)));
                            s.version.borrow_mut().set_text(msg_component);
                        }
                        if let Some(favicon) = res.favicon {
                            let name: String = std::iter::repeat(())
                                .map(|()| {
                                    rand::thread_rng().sample(rand::distributions::Alphanumeric)
                                        as char
                                })
                                .take(30)
                                .collect();
                            let tex = renderer.get_textures_ref();
                            s.icon_texture = Some(name.clone());
                            let icon_tex = tex.write().put_dynamic(&name, favicon);
                            s.icon.borrow_mut().texture = icon_tex.name;
                        }
                    }
                    Err(TryRecvError::Disconnected) => {
                        s.done_ping = true;
                        s.motd
                            .borrow_mut()
                            .set_text(Component::new(ComponentType::new(
                                "Channel dropped",
                                Some(format::Color::Red),
                            )));
                    }
                    _ => {}
                }
            }
        }
    }

    fn on_scroll(&mut self, _: f64, y: f64) {
        let elements = self.elements.as_mut().unwrap();
        if elements.servers.is_empty() {
            return;
        }
        let mut diff = y / 1.0;
        {
            let last = elements.servers.last().unwrap();
            if last.offset + diff <= 2.0 {
                diff = 2.0 - last.offset;
            }
            let first = elements.servers.first().unwrap();
            if first.offset + diff >= 0.0 {
                diff = -first.offset;
            }
        }

        for s in &mut elements.servers {
            s.offset += diff;
            s.update_position();
        }
    }

    fn on_resize(
        &mut self,
        screen_sys: &Arc<ScreenSystem>,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        // TODO: Don't ping the servers on resize!
        self.on_deactive(screen_sys, renderer, ui_container);
        self.on_active(screen_sys, renderer, ui_container);
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}
