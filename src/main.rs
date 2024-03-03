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

#![recursion_limit = "300"]
#![allow(clippy::too_many_arguments)] // match standard gl functions with many arguments
#![allow(clippy::many_single_char_names)] // short variable names provide concise clarity
#![allow(clippy::float_cmp)] // float comparison used to check if changed

mod console;
use copypasta::nop_clipboard;
use copypasta::ClipboardContext;
use copypasta::ClipboardProvider;
use glutin::config::Api;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::ContextApi;
use glutin::context::ContextAttributesBuilder;
use glutin::context::GlContext;
use glutin::context::NotCurrentGlContext;
use glutin::display::GetGlDisplay;
use glutin::display::GlDisplay;
use glutin::surface::GlSurface;
use glutin::surface::SwapInterval;
use glutin_winit::DisplayBuilder;
use glutin_winit::GlWindow;
#[cfg(target_os = "linux")]
use instant::{Duration, Instant};
use leafish_protocol::protocol::login::AccountType;
use log::{debug, error, info, warn};
use raw_window_handle::HasRawWindowHandle;
use shared::Version;
use std::fs;
use std::num::NonZeroU32;
use winit::keyboard::Key;
use winit::keyboard::ModifiersKeyState;
use winit::keyboard::NamedKey;
use winit::keyboard::SmolStr;
use winit::raw_window_handle::HasDisplayHandle;
use winit::raw_window_handle::RawDisplayHandle;
use winit::window::Icon;
extern crate leafish_shared as shared;

use structopt::StructOpt;

extern crate leafish_protocol;

pub mod ecs;
use leafish_protocol::format;
use leafish_protocol::nbt;
use leafish_protocol::protocol;
pub mod gl;
use leafish_protocol::types;
pub mod chunk_builder;
pub mod entity;
mod inventory;
pub mod model;
pub mod particle;
pub mod paths;
pub mod render;
pub mod resources;
pub mod screen;
pub mod server;
pub mod settings;
pub mod ui;
pub mod world;

use crate::entity::Rotation;
use crate::render::hud::HudContext;
use crate::settings::KeybindStore;
use crate::settings::SettingStore;
use crate::settings::SettingType;
use leafish_protocol::protocol::login::Account;
use leafish_protocol::protocol::Error;
use parking_lot::Mutex;
use parking_lot::RwLock;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;

// TODO: Improve calculate light performance and fix capturesnapshot

pub struct Game {
    renderer: Arc<render::Renderer>,
    screen_sys: Arc<screen::ScreenSystem>,
    resource_manager: Arc<RwLock<resources::Manager>>,
    clipboard_provider: Mutex<Box<dyn copypasta::ClipboardProvider>>,
    console: Arc<Mutex<console::Console>>,
    settings: Rc<settings::SettingStore>,
    keybinds: Rc<settings::KeybindStore>,
    should_close: bool,

    server: Option<Arc<server::Server>>,
    focused: bool,
    chunk_builder: chunk_builder::ChunkBuilder,

    connect_error: Option<Error>,

    dpi_factor: f64,
    last_mouse_x: f64,
    last_mouse_y: f64,
    last_mouse_xrel: f64,
    last_mouse_yrel: f64,
    is_ctrl_pressed: bool,
    is_logo_pressed: bool,
    is_fullscreen: bool,
    default_protocol_version: i32,
    current_account: Arc<Mutex<Option<Account>>>,
}

impl Game {
    pub fn connect_to(
        &mut self,
        address: &str,
        hud_context: Arc<RwLock<HudContext>>,
    ) -> Result<(), Error> {
        let (protocol_version, forge_mods, fml_network_version) =
            match protocol::Conn::new(address, self.default_protocol_version)
                .and_then(|conn| conn.do_status())
            {
                Ok(res) => {
                    info!(
                        "Detected server protocol version {}",
                        res.0.version.protocol
                    );
                    (
                        res.0.version.protocol,
                        res.0.forge_mods,
                        res.0.fml_network_version,
                    )
                }
                Err(err) => {
                    warn!(
                        "Error pinging server {} to get protocol version: {:?}, defaulting to {}",
                        address, err, self.default_protocol_version
                    );
                    (self.default_protocol_version, vec![], None)
                }
            };
        if !Version::from_id(protocol_version as u32).is_supported() {
            return Err(Error::Err(format!(
                "The server's version isn't supported!\n(protocol version: {})",
                protocol_version
            )));
        }
        let address = address.to_owned();
        let resources = self.resource_manager.clone();
        let renderer = self.renderer.clone();
        let screen_sys = self.screen_sys.clone();
        let account = self.current_account.clone();
        let result = thread::spawn(move || {
            server::Server::connect(
                resources,
                account.lock().as_ref().unwrap(),
                &address,
                protocol_version,
                forge_mods,
                fml_network_version,
                renderer,
                hud_context.clone(),
                screen_sys,
            )
        })
        .join();
        match result {
            Ok(result) => {
                match result {
                    Ok(srv) => {
                        self.server = Some(srv);
                        Ok(())
                    }
                    Err(err) => {
                        let str = err.to_string();
                        self.connect_error = Some(err);
                        // self.server.disconnect_reason = Some(Component::from_string(&*err.to_string()));
                        Err(Error::Err(str))
                    }
                }
            }
            Err(_) => Err(Error::Err("Unknown".to_string())),
        }
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "leafish")]
struct Opt {
    /// Log decoded packets received from network
    #[structopt(short = "n", long = "network-debug")]
    network_debug: bool,

    /// Parse a network packet from a file
    #[structopt(short = "N", long = "network-parse-packet")]
    network_parse_packet: Option<String>,

    /// Protocol version to use in the autodetection ping
    #[structopt(short = "p", long = "default-protocol-version")]
    default_protocol_version: Option<String>,
    #[structopt(long)]
    uuid: Option<String>,
    #[structopt(long)]
    name: Option<String>,
    #[structopt(long)]
    token: Option<String>,
}

// TODO: Hide own character and show only the right hand. (with an item)
// TODO: Simplify error messages in server list.
// TODO: Render skin of players joining after one self.
// TODO: Implement arm swing animation!
// TODO: Implement attacking entities!
// TODO: Fix cursor grabbing/visibility/transparency of window.
// TODO: Improve clouds.
// TODO: Fix pistons.
fn main() {
    let opt = Opt::from_args();
    #[allow(clippy::arc_with_non_send_sync)]
    let con = Arc::new(Mutex::new(console::Console::new()));
    let proxy = console::ConsoleProxy::new(con.clone());

    log::set_boxed_logger(Box::new(proxy)).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    info!("Starting Leafish...");

    let settings = Rc::new(SettingStore::new());
    let keybinds = Rc::new(KeybindStore::new());
    info!("settings all loaded!");

    con.lock().configure(&settings);
    let vsync = settings.get_bool(SettingType::Vsync);

    let (res, mut resui) = resources::Manager::new();
    let resource_manager = Arc::new(RwLock::new(res));

    let events_loop = winit::event_loop::EventLoop::new().unwrap();

    let window_builder = winit::window::WindowBuilder::new()
        .with_title("Leafish")
        .with_window_icon(Some(
            Icon::from_rgba(
                image::load_from_memory(include_bytes!("../resources/icon32x32.png"))
                    .unwrap()
                    .into_rgba8()
                    .into_vec(),
                32,
                32,
            )
            .unwrap(),
        ))
        .with_inner_size(winit::dpi::LogicalSize::new(854.0, 480.0)) // FIXME: Why are we using this particular value here?
        .with_maximized(true);

    let (context, shader_version, dpi_factor, window, surface, display) = {
        let template = ConfigTemplateBuilder::new()
            .with_stencil_size(0)
            .with_depth_size(24)
            .with_api(Api::GLES3.union(Api::OPENGL));
        let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));

        let (window, gl_config) = display_builder
            .build(&events_loop, template, |mut configs| {
                configs.next().unwrap()
            })
            .unwrap();

        let raw_window_handle = window.as_ref().map(|window| window.raw_window_handle());
        let gl_display = gl_config.display();
        let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);

        let fallback_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(raw_window_handle);

        let not_current_gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .unwrap_or_else(|_| {
                    gl_display
                        .create_context(&gl_config, &fallback_context_attributes)
                        .expect("failed to create context")
                })
        };

        let shader_version = match not_current_gl_context.context_api() {
            ContextApi::OpenGl(_) => "#version 150",  // OpenGL 3.2
            ContextApi::Gles(_) => "#version 300 es", // OpenGL ES 3.0 (similar to WebGL 2)
        };

        let window = window.unwrap();

        let attrs = window.build_surface_attributes(Default::default());
        let gl_surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };

        // Make it current.
        let gl_context = not_current_gl_context.make_current(&gl_surface).unwrap();

        if vsync {
            // Try setting vsync.
            if let Err(res) = gl_surface
                .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
            {
                eprintln!("Error setting vsync: {res:?}");
            }
        }

        (
            gl_context,
            shader_version,
            window.scale_factor(),
            window,
            gl_surface,
            gl_display,
        )
    };

    gl::init(&display);
    info!("Shader version: {}", shader_version);

    let renderer = render::Renderer::new(resource_manager.clone(), shader_version);
    let ui_container = ui::Container::new();

    let mut last_frame = Instant::now();

    let screen_sys = Arc::new(screen::ScreenSystem::new());
    let active_account = Arc::new(Mutex::new(None));
    screen_sys.add_screen(Box::new(screen::background::Background::new(
        settings.clone(),
        screen_sys.clone(),
    )));
    let mut accounts = screen::launcher::load_accounts().unwrap_or_default();
    if let Some((name, uuid, token)) = opt
        .name
        .clone()
        .and_then(|name| {
            opt.uuid
                .clone()
                .map(|uuid| opt.token.clone().map(|token| (name, uuid, token)))
        })
        .flatten()
    {
        println!("Got microsoft credentials, adding account...");
        accounts.push(Account {
            name: name.clone(),
            uuid: Some(uuid),
            verification_tokens: vec![name, "".to_string(), token],
            head_img_data: None,
            account_type: AccountType::Microsoft,
        });
    }
    screen_sys.add_screen(Box::new(screen::launcher::Launcher::new(
        Arc::new(Mutex::new(accounts)),
        screen_sys.clone(),
        active_account.clone(),
    )));

    let textures = renderer.get_textures();
    let default_protocol_version = protocol::versions::protocol_name_to_protocol_version(
        opt.default_protocol_version.unwrap_or_default(),
    );

    #[cfg(target_os = "linux")]
    let clipboard: Box<dyn ClipboardProvider> = match events_loop.display_handle() {
        Ok(display) => {
            match display.as_raw() {
                RawDisplayHandle::Wayland(wayland) => {
                    debug!("Configured with wayland clipboard");
                    // NOTE: Since this takes a pointer to the winit event loop, it MUST be dropped first.
                    unsafe {
                        Box::new(
                            copypasta::wayland_clipboard::create_clipboards_from_external(
                                wayland.display.as_ptr(),
                            )
                            .1,
                        )
                    }
                }
                _ => create_clipboard(),
            }
        }
        Err(_) => create_clipboard(),
    };

    #[cfg(not(target_os = "linux"))]
    let clipboard = create_clipboard();

    let game = Game {
        server: None,
        focused: false,
        renderer: Arc::new(renderer),
        screen_sys,
        resource_manager: resource_manager.clone(),
        console: con,
        should_close: false,
        chunk_builder: chunk_builder::ChunkBuilder::new(resource_manager, textures),
        connect_error: None,
        dpi_factor,
        last_mouse_x: 0.0,
        last_mouse_y: 0.0,
        last_mouse_xrel: 0.0,
        last_mouse_yrel: 0.0,
        is_ctrl_pressed: false,
        is_logo_pressed: false,
        is_fullscreen: false,
        default_protocol_version,
        clipboard_provider: Mutex::new(clipboard),
        current_account: active_account,
        settings,
        keybinds,
    };
    if opt.network_debug {
        protocol::enable_network_debug();
    }

    if let Some(filename) = opt.network_parse_packet {
        let data = fs::read(filename).unwrap();
        protocol::try_parse_packet(data, default_protocol_version);
        return;
    }

    let mut last_resource_version = 0;

    let game = Rc::new(RefCell::new(game));
    let ui_container = Rc::new(RefCell::new(ui_container));

    let game = Rc::clone(&game);
    let ui_container = Rc::clone(&ui_container);
    events_loop
        .run(move |event, event_loop| {
            let mut game = game.borrow_mut();
            let mut ui_container = ui_container.borrow_mut();
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

            if let winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::Resized(physical_size),
                ..
            } = event
            {
                surface.resize(
                    &context,
                    NonZeroU32::new(physical_size.width).unwrap(),
                    NonZeroU32::new(physical_size.height).unwrap(),
                );
            }

            if !handle_window_event(&window, &mut game, &mut ui_container, event) {
                return;
            }

            let start = Instant::now();
            tick_all(
                &window,
                &mut game,
                &mut ui_container,
                &mut last_frame,
                &mut resui,
                &mut last_resource_version,
                vsync,
            );
            if DEBUG {
                let dist = Instant::now().checked_duration_since(start);
                debug!("Ticking took {}", dist.unwrap().as_millis());
            }
            surface
                .swap_buffers(&context)
                .expect("Failed to swap GL buffers");

            if game.should_close {
                event_loop.exit();
            }
        })
        .unwrap();
}

const DEBUG: bool = false;

fn tick_all(
    window: &winit::window::Window,
    game: &mut Game,
    ui_container: &mut ui::Container,
    last_frame: &mut Instant,
    resui: &mut resources::ManagerUI,
    last_resource_version: &mut usize,
    vsync: bool,
) {
    if game.server.is_some() {
        if !game.server.as_ref().unwrap().is_connected() {
            let disconnect_reason = game
                .server
                .as_ref()
                .unwrap()
                .disconnect_data
                .write()
                .disconnect_reason
                .take();
            game.screen_sys.close_closable_screens();
            game.screen_sys
                .replace_screen(Box::new(screen::ServerList::new(disconnect_reason)));
            game.server = None;
            game.renderer.reset();
        } else if game
            .server
            .as_ref()
            .unwrap()
            .disconnect_gracefully
            .load(Ordering::Relaxed)
        {
            game.server.as_ref().unwrap().finish_disconnect();
            let disconnect_reason = game
                .server
                .as_ref()
                .unwrap()
                .disconnect_data
                .write()
                .disconnect_reason
                .take();
            game.screen_sys.close_closable_screens();
            game.screen_sys
                .replace_screen(Box::new(screen::ServerList::new(disconnect_reason)));
            game.server = None;
            game.renderer.reset();
        }
    } else {
        game.chunk_builder.reset();
    }
    let now = Instant::now();
    let diff = now.duration_since(*last_frame);
    *last_frame = now;
    let frame_time = 1e9f64 / 60.0;
    let delta = (diff.subsec_nanos() as f64) / frame_time;
    let physical_size = window.inner_size();
    let (physical_width, physical_height) = physical_size.into();
    let (width, height): (u32, u32) = physical_size.to_logical::<f64>(game.dpi_factor).into();

    let version = {
        let try_res = game.resource_manager.try_write();
        if let Some(mut res) = try_res {
            res.tick(resui, ui_container, delta);
            res.version()
        } else {
            // TODO: why does game.resource_manager.write() sometimes deadlock?
            warn!("Failed to obtain mutable reference to resource manager!"); // was uncommented
            *last_resource_version
        }
    };
    *last_resource_version = version;

    let vsync_changed = game.settings.get_bool(SettingType::Vsync);
    if vsync != vsync_changed {
        error!("Changing vsync currently requires restarting");
        game.should_close = true;
        // TODO: after changing to wgpu and the new renderer, allow changing vsync on a Window
        //vsync = vsync_changed;
    }
    let fps_cap = game.settings.get_i32(SettingType::MaxFps);

    if game.server.is_some() {
        game.server.as_ref().unwrap().clone().tick(delta, game); // TODO: Improve perf in load screen!
    }

    // Check if window is valid, it might be minimized
    if physical_width == 0 || physical_height == 0 {
        return;
    }

    if game.server.is_some() {
        game.renderer.update_camera(physical_width, physical_height);
        game.chunk_builder.tick(
            game.server.as_ref().unwrap().world.clone(),
            game.renderer.clone(),
            version,
        );
    } else if game.renderer.screen_data.read().safe_width != physical_width
        || game.renderer.screen_data.read().safe_height != physical_height
    {
        game.renderer.screen_data.write().safe_width = physical_width;
        game.renderer.screen_data.write().safe_height = physical_height;
        gl::viewport(0, 0, physical_width as i32, physical_height as i32);
    }

    if game
        .screen_sys
        .tick(delta, game.renderer.clone(), ui_container, window)
    {
        window
            .set_cursor_grab(winit::window::CursorGrabMode::None)
            .unwrap();
        window.set_cursor_visible(true);
        game.focused = false;
    } else {
        // see https://docs.rs/winit/latest/winit/window/enum.CursorGrabMode.html
        // fix for https://github.com/Lea-fish/Leafish/issues/265
        let cursor_grab_mode = if cfg!(target_os = "macos") {
            winit::window::CursorGrabMode::Locked
        } else {
            winit::window::CursorGrabMode::Confined
        };
        window.set_cursor_grab(cursor_grab_mode).unwrap();
        window.set_cursor_visible(false);
        game.focused = true;
    }
    game.console
        .lock()
        .tick(ui_container, game.renderer.clone(), delta, width as f64);
    ui_container.tick(game.renderer.clone(), delta, width as f64, height as f64);
    let world = game.server.as_ref().map(|server| server.world.clone());
    game.renderer
        .clone()
        .tick(world, delta, width, height, physical_width, physical_height);
    if game.server.is_some() {
        game.server
            .as_ref()
            .unwrap()
            .render_list_computer
            .send(true)
            .unwrap();
    }

    if fps_cap > 0 && !vsync {
        let frame_time = now.elapsed();
        let sleep_interval = Duration::from_millis(1000 / fps_cap as u64);
        if frame_time < sleep_interval {
            thread::sleep(sleep_interval - frame_time);
        }
    }
}
// TODO: Improve perf of 3, 6 and 10
// TODO: Reenable: [server/mod.rs:1924][WARN] Block entity at (1371,53,-484) missing id tag: NamedTag("", Compound({"y": Int(53), "Sign": String(""), "x": Int(1371), "z": Int(-484)}))

fn handle_window_event<T>(
    window: &winit::window::Window,
    game: &mut Game,
    ui_container: &mut ui::Container,
    event: winit::event::Event<T>,
) -> bool {
    use winit::event::*;
    match event {
        Event::AboutToWait => return true,
        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion {
                delta: (xrel, yrel),
            },
            ..
        } => {
            let mouse_sens: f64 = game.settings.get_float(SettingType::MouseSense);
            let (rx, ry) = if xrel > 1000.0 || yrel > 1000.0 {
                // Heuristic for if we were passed an absolute value instead of relative
                // Workaround https://github.com/tomaka/glutin/issues/1084 MouseMotion event returns absolute instead of relative values, when running Linux in a VM
                // Note SDL2 had a hint to handle this scenario:
                // sdl2::hint::set_with_priority("SDL_MOUSE_RELATIVE_MODE_WARP", "1", &sdl2::hint::Hint::Override);
                let s = 8000.0 + 0.01;
                (
                    ((xrel - game.last_mouse_xrel) / s) * mouse_sens,
                    ((yrel - game.last_mouse_yrel) / s) * mouse_sens,
                )
            } else {
                let s = 2000.0 + 0.01;
                ((xrel / s) * mouse_sens, (yrel / s) * mouse_sens)
            };

            game.last_mouse_xrel = xrel;
            game.last_mouse_yrel = yrel;

            use std::f64::consts::PI;

            if game.focused {
                // see https://docs.rs/winit/latest/winit/window/enum.CursorGrabMode.html
                // fix for https://github.com/Lea-fish/Leafish/issues/265
                let cursor_grab_mode = if cfg!(target_os = "macos") {
                    winit::window::CursorGrabMode::Locked
                } else {
                    winit::window::CursorGrabMode::Confined
                };
                window.set_cursor_grab(cursor_grab_mode).unwrap();
                window.set_cursor_visible(false);
                if game.server.is_some()
                    && !game.server.as_ref().unwrap().dead.load(Ordering::Acquire)
                {
                    if let Some(player) = *game.server.as_ref().unwrap().player.write() {
                        let server = game.server.as_ref().unwrap();
                        let entities = server.entities.clone();
                        let mut entities = entities.write();
                        let mut rotation = entities
                            .world
                            .entity_mut(player.1)
                            .get_mut::<Rotation>()
                            .unwrap();
                        rotation.yaw -= rx;
                        rotation.pitch -= ry;
                        if rotation.pitch < (PI / 2.0) + 0.01 {
                            rotation.pitch = (PI / 2.0) + 0.01;
                        }
                        if rotation.pitch > (PI / 2.0) * 3.0 - 0.01 {
                            rotation.pitch = (PI / 2.0) * 3.0 - 0.01;
                        }
                    }
                }
            } else {
                window
                    .set_cursor_grab(winit::window::CursorGrabMode::None)
                    .unwrap();
                window.set_cursor_visible(true);
            }
        }

        Event::WindowEvent { event, .. } => {
            match event {
                WindowEvent::ModifiersChanged(modifiers_state) => {
                    game.is_ctrl_pressed = modifiers_state.lcontrol_state()
                        == ModifiersKeyState::Pressed
                        || modifiers_state.rcontrol_state() == ModifiersKeyState::Pressed;
                    game.is_logo_pressed = modifiers_state.lsuper_state()
                        == ModifiersKeyState::Pressed
                        || modifiers_state.rsuper_state() == ModifiersKeyState::Pressed;
                }
                WindowEvent::CloseRequested => game.should_close = true,
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    game.dpi_factor = scale_factor;
                }

                WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                    (ElementState::Released, MouseButton::Left) => {
                        let physical_size = window.inner_size();
                        let (width, height) =
                            physical_size.to_logical::<f64>(game.dpi_factor).into();
                        if !game.screen_sys.clone().is_current_ingame() && !game.focused {
                            // TODO: after Pointer Lock https://github.com/rust-windowing/winit/issues/1674
                            ui_container.click_at(
                                game,
                                game.last_mouse_x,
                                game.last_mouse_y,
                                width,
                                height,
                            );
                        }
                        if let Some(server) = &game.server {
                            server.on_release_left_click(game.focused);
                        }
                    }
                    (ElementState::Pressed, MouseButton::Left) => {
                        if let Some(server) = &game.server {
                            server.on_left_click(game.focused);
                        }
                    }
                    (ElementState::Released, MouseButton::Right) => {
                        if let Some(server) = &game.server {
                            server.on_release_right_click(game.focused);
                        }
                    }
                    (ElementState::Pressed, MouseButton::Right) => {
                        if let Some(server) = &game.server {
                            server.on_right_click(game.focused);
                        }
                    }
                    (_, _) => (),
                },
                WindowEvent::CursorMoved { position, .. } => {
                    let (x, y) = position.to_logical::<f64>(game.dpi_factor).into();
                    game.last_mouse_x = x;
                    game.last_mouse_y = y;

                    if !game.focused {
                        let physical_size = window.inner_size();
                        let (width, height) =
                            physical_size.to_logical::<f64>(game.dpi_factor).into();
                        ui_container.hover_at(game, x, y, width, height);
                        if let Some(server) = &game.server {
                            server.on_cursor_moved(x, y);
                        }
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    // TODO: line vs pixel delta? does pixel scrolling (e.g. touchpad) need scaling?
                    match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            game.screen_sys.on_scroll(x.into(), y.into());
                        }
                        MouseScrollDelta::PixelDelta(position) => {
                            let (x, y) = position.into();
                            game.screen_sys.on_scroll(x, y);
                        }
                    }
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    const SEMICOLON: Key = Key::Character(SmolStr::new_inline(";"));
                    if event.state == ElementState::Pressed && event.logical_key == SEMICOLON {
                        game.console.lock().toggle();
                    } else {
                        match (event.state, event.logical_key) {
                            (ElementState::Pressed, Key::Named(NamedKey::F11)) => {
                                if !game.is_fullscreen {
                                    // TODO: support options for exclusive and simple fullscreen
                                    // see https://docs.rs/glutin/0.22.0-alpha5/glutin/window/struct.Window.html#method.set_fullscreen
                                    window.set_fullscreen(Some(
                                        winit::window::Fullscreen::Borderless(
                                            window.current_monitor(),
                                        ),
                                    ));
                                } else {
                                    window.set_fullscreen(None);
                                }

                                game.is_fullscreen = !game.is_fullscreen;
                            }
                            (ElementState::Pressed, key) => {
                                #[cfg(target_os = "macos")]
                                if game.is_logo_pressed && key.eq_ignore_case('q') {
                                    game.should_close = true;
                                }
                                if !game.focused {
                                    let ctrl_pressed = game.is_ctrl_pressed || game.is_logo_pressed;
                                    ui_container.key_press(game, key.clone(), true, ctrl_pressed);
                                }
                                game.screen_sys.clone().press_key(
                                    (key, event.physical_key),
                                    true,
                                    game,
                                );
                            }
                            (ElementState::Released, key) => {
                                if !game.focused {
                                    let ctrl_pressed = game.is_ctrl_pressed;
                                    ui_container.key_press(game, key.clone(), false, ctrl_pressed);
                                }
                                game.screen_sys.clone().press_key(
                                    (key, event.physical_key),
                                    false,
                                    game,
                                );
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        _ => (),
    }

    false
}

fn create_clipboard() -> Box<dyn ClipboardProvider> {
    match ClipboardContext::new() {
        Ok(clipboard) => {
            debug!("Configured with normal clipboard");
            Box::new(clipboard)
        }
        Err(_) => {
            debug!("Could not create clipboard running with no operation clipboard");
            Box::new(
                nop_clipboard::NopClipboardContext::new()
                    .expect("no operation clipboard can never fail"),
            )
        }
    }
}

pub trait KeyCmp {
    fn eq_ignore_case(&self, other: char) -> bool;
}

impl KeyCmp for Key {
    fn eq_ignore_case(&self, other: char) -> bool {
        match self {
            Key::Character(content) => {
                if content.as_str().len() != 1 {
                    return false;
                }
                let chr = content.as_str().chars().next().unwrap();
                if !other.is_alphabetic() {
                    return chr == other;
                }
                chr.to_ascii_lowercase() == other || chr.to_ascii_uppercase() == other
            }
            _ => false,
        }
    }
}
