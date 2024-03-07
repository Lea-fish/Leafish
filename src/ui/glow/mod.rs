use std::{collections::VecDeque, num::NonZeroU32, sync::{atomic::Ordering, Arc, OnceLock}, thread, time::Instant};

use arc_swap::{access::Access, ArcSwapOption};
use bevy_ecs::system::Resource;
use cgmath::InnerSpace;
use collision::Frustum;
use crossbeam_channel::{unbounded, Receiver, Sender};
use glutin::{config::{Api, ConfigTemplateBuilder}, context::{ContextApi, ContextAttributesBuilder, GlContext, NotCurrentGlContext}, display::{GetGlDisplay, GlDisplay}, surface::{GlSurface, SwapInterval}};
use glutin_winit::{DisplayBuilder, GlWindow};
use instant::Duration;
use log::{debug, info, warn};
use parking_lot::{Mutex, RwLock};
use raw_window_handle::HasRawWindowHandle;
use shared::Direction;
use winit::{event_loop::{EventLoopBuilder, EventLoopProxy}, keyboard::{Key, ModifiersKeyState, NamedKey, SmolStr}, window::{CursorGrabMode, Icon, Window}};

use render::Renderer;

use crate::{ecs, server::Server, ui::glow::{entity::Rotation, screen::{background::Background, ServerList}, ui::Container}, world::{CPos, World}, Game, DEBUG};

use self::{inventory::InventoryContext, render::{chunk_builder::{self, ChunkBuilder, CullInfo}, hud::HudContext, sun::SunModel, target::Info}, screen::{chat::ChatContext, ScreenSystem}, ui::resources::ManagerUI};

use super::{InterUiMessage, UiQueue};

pub mod ui;
pub mod render;
pub mod screen;
pub mod entity;
pub mod gl;
pub mod inventory;
pub mod model;
pub mod particle;

pub(crate) struct UiCtx {
    pub(crate) screen_sys: Arc<ScreenSystem>,
    pub(crate) renderer: Arc<Renderer>,
    pub(crate) window: Arc<Window>,
    pub(crate) queue: EventLoopProxy<InterUiMessage>,
    pub(crate) clipboard_provider: Mutex<Box<dyn copypasta::ClipboardProvider>>,
    pub(crate) input_ctx: RwLock<InputCtx>,
    pub(crate) server_ctx: ArcSwapOption<ServerCtx>,
    pub(crate) hud_context: Arc<RwLock<HudContext>>,
    pub(crate) inventory_context: Arc<RwLock<InventoryContext>>,
    pub(crate) chat_ctx: Arc<ChatContext>,
}

pub(crate) struct InputCtx {
    // FIXME: make these atomic!
    pub dpi_factor: f64,
    pub last_mouse_x: f64,
    pub last_mouse_y: f64,
    pub last_mouse_xrel: f64,
    pub last_mouse_yrel: f64,
    pub is_ctrl_pressed: bool,
    pub is_logo_pressed: bool,
    pub is_fullscreen: bool,
    pub focused: bool,
}

pub(crate) struct ServerCtx {
    server: Arc<ArcSwapOption<Server>>,
    sun_model: RwLock<Option<SunModel>>,
    target_info: Arc<RwLock<Info>>,
    // this is the chunk render list
    render_list: Arc<RwLock<Vec<(i32, i32, i32)>>>,
    chunk_builder: Mutex<ChunkBuilder>,
}

static UI_CTX: OnceLock<Arc<UiCtx>> = OnceLock::new();

pub(crate) fn ctx() -> &'static Arc<UiCtx> {
    UI_CTX.get().unwrap()
}

pub fn queue() -> Box<dyn UiQueue> {
    Box::new(|msg| {
        ctx().queue.send_event(msg).unwrap();
    })
}

pub fn run(game: &Arc<Game>) -> anyhow::Result<()> {
    let events_loop = EventLoopBuilder::<InterUiMessage>::with_user_event().build()?;

    let window_builder = winit::window::WindowBuilder::new()
        .with_title("Leafish")
        .with_window_icon(Some(
            Icon::from_rgba(
                image::load_from_memory(include_bytes!("../../../resources/icon32x32.png"))
                    .unwrap()
                    .into_rgba8()
                    .into_vec(),
                32,
                32,
            )?,
        ))
        // we use these values as they are also the default values vanilla minecraft uses
        .with_inner_size(winit::dpi::LogicalSize::new(854.0, 480.0))
        .with_maximized(true);

    let vsync = game.settings.get_bool(crate::BoolSetting::Vsync);

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

    let renderer = Renderer::new(game.resource_manager.clone(), shader_version);
    let mut ui_container = Container::new();

    // FIXME: init UI_CTX

    let mut last_frame = Instant::now();

    let screen_sys = Arc::new(ScreenSystem::new());
    screen_sys.add_screen(Box::new(Background::new(
        game.settings.clone(),
        screen_sys.clone(),
    )));

    // FIXME: add launcher


    let mut resui = ManagerUI::default();
    let mut last_resource_version = 0;
    let fps_cap = game.settings.get_int(crate::IntSetting::MaxFps);

    events_loop
        .run(move |event, event_loop| {
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

            if !handle_window_event(&game, &window, &mut ui_container, event) {
                return;
            }

            let start = Instant::now();
            tick_all(
                &game,
                &window,
                &mut ui_container,
                &mut last_frame,
                &mut resui,
                &mut last_resource_version,
                vsync,
                fps_cap as usize,
            );
            if DEBUG {
                let dist = Instant::now().checked_duration_since(start);
                debug!("Ticking took {}", dist.unwrap().as_millis());
            }
            surface
                .swap_buffers(&context)
                .expect("Failed to swap GL buffers");

            if game.should_close.load(Ordering::Acquire) {
                event_loop.exit();
            }
        })?;
        Ok(())
}

pub(crate) fn init_systems(manager: &mut ecs::Manager) -> anyhow::Result<()> {
    let ctx = ctx();
    manager.world.insert_resource(entity::GameInfo::new());
    manager
        .world
        .insert_resource(RendererResource(ctx.renderer.clone()));
    manager
        .world
        .insert_resource(ScreenSystemResource(ctx.screen_sys.clone()));
    manager
        .world
        .insert_resource(InventoryContextResource(ctx.inventory_context.clone()));
    ctx.hud_context.write().server = Some(ctx.server_ctx.load().as_ref().unwrap().server.load().as_ref().unwrap().clone());
    entity::add_systems(manager);
    Ok(())
}

fn tick_all(
    game: &Arc<Game>,
    window: &winit::window::Window,
    ui_container: &mut ui::Container,
    last_frame: &mut Instant,
    resui: &mut ManagerUI,
    last_resource_version: &mut usize,
    vsync: bool,
    fps_cap: usize,
) {
    let ctx = ctx();
    if let Some(server) = ctx.server_ctx.load().as_ref() {
        if !server.server.load().as_ref().unwrap().is_connected() {
            let disconnect_reason = server.server.load().as_ref().unwrap()
                .disconnect_data
                .write()
                .disconnect_reason
                .take();
            server.server.store(None);
            server.chunk_builder.lock().reset();
            game.send_ui_msg(InterUiMessage::Disconnected { reason: disconnect_reason });
        } else if server.server.load().as_ref().unwrap()
            .disconnect_gracefully
            .load(Ordering::Acquire)
        {
            server.server.load().as_ref().unwrap().finish_disconnect();
            let disconnect_reason = server.server.load().as_ref().unwrap()
                .disconnect_data
                .write()
                .disconnect_reason
                .take();
            server.server.store(None);
            server.chunk_builder.lock().reset();
            game.send_ui_msg(InterUiMessage::Disconnected { reason: disconnect_reason });
        }
    }
    let now = Instant::now();
    let diff = now.duration_since(*last_frame);
    *last_frame = now;
    let frame_time = 1e9f64 / 60.0;
    let delta = (diff.subsec_nanos() as f64) / frame_time;
    let physical_size = window.inner_size();
    let (physical_width, physical_height) = physical_size.into();
    let (width, height): (u32, u32) = physical_size.to_logical::<f64>(ctx.input_ctx.read().dpi_factor).into();

    let version = {
        let try_res = game.resource_manager.try_write();
        if let Some(mut res) = try_res {
            res.tick();
            res.version()
        } else {
            // TODO: why does game.resource_manager.write() sometimes deadlock?
            warn!("Failed to obtain mutable reference to resource manager!"); // was uncommented
            *last_resource_version
        }
    };
    *last_resource_version = version;

    // FIXME: support changing vsync!
    /*let vsync_changed = *game.vars.get(settings::R_VSYNC);
    if *vsync != vsync_changed {
        error!("Changing vsync currently requires restarting");
        game.should_close = true;
        // TODO: after changing to wgpu and the new renderer, allow changing vsync on a Window
        //vsync = vsync_changed;
    }*/
    // FIXME: support changing fps_cap
    // let fps_cap = *game.vars.get(settings::R_MAX_FPS);

    if let Some(server) = ctx.server_ctx.load().as_ref().map(|server| server.as_ref().server.load().as_ref().cloned()).flatten() {
        server.tick(delta); // TODO: Improve perf in load screen!
    }

    // Check if window is valid, it might be minimized
    if physical_width == 0 || physical_height == 0 {
        return;
    }

    if let Some(server) = ctx.server_ctx.load().as_ref() {
        let actual_server = server.server.load();
        if let Some(actual_server) = actual_server.as_ref() {
            ctx.renderer.update_camera(physical_width, physical_height);
            server.chunk_builder.lock()
                .tick(actual_server.world.clone(), ctx.renderer.clone(), version);
        }
    }
    if ctx.renderer.screen_data.read().safe_width != physical_width
        || ctx.renderer.screen_data.read().safe_height != physical_height
    {
        ctx.renderer.screen_data.write().safe_width = physical_width;
        ctx.renderer.screen_data.write().safe_height = physical_height;
        gl::viewport(0, 0, physical_width as i32, physical_height as i32);
    }

    let world = if let Some(server) = ctx.server_ctx.load().as_ref() {
        if let Some(actual_server) = server.server.load().as_ref() {
            let world = actual_server.world.clone();
            
            /*server
                    .render_list_computer
                    .send(true)
                    .unwrap();*/
            Some(world)
        } else {
            None
        }
    } else {
        None
    };
    ctx.renderer
                .tick(world, delta, width, height, physical_width, physical_height);

    render_ui(delta, width, height, ui_container);

    if fps_cap > 0 && !vsync {
        let frame_time = now.elapsed();
        let sleep_interval = Duration::from_millis(1000 / fps_cap as u64);
        if frame_time < sleep_interval {
            thread::sleep(sleep_interval - frame_time);
        }
    }
}

fn render_ui(delta: f64, width: u32, height: u32, ui_container: &mut Container) {
    let ctx = ctx();
    let mut input = ctx.input_ctx.write();
    if ctx
        .screen_sys
        .tick(delta, &ctx.renderer, ui_container, &ctx.window)
    {
        if input.focused {
            ctx.window
                .set_cursor_grab(winit::window::CursorGrabMode::None)
                .unwrap();
            ctx.window.set_cursor_visible(true);
            input.focused = false;
        }
    } else if !input.focused {
        // see https://docs.rs/winit/latest/winit/window/enum.CursorGrabMode.html
        // fix for https://github.com/Lea-fish/Leafish/issues/265
        // prefer Locked cursor mode, and fallback to Confined if that doesn't work
        if ctx.window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
            ctx.window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
        }
        ctx.window.set_cursor_visible(false);
        input.focused = true;
    }
    // FIXME: tick console!
    /*game.console
        .lock()
        .tick(ui_container, ctx.renderer.clone(), delta, width as f64);*/
    ui_container.tick(&ctx.renderer, delta, width as f64, height as f64);
}

fn render_ingame(actual_server: &Arc<Server>, server: &Arc<ServerCtx>) {

}

// TODO: Improve perf of 3, 6 and 10
// TODO: Reenable: [server/mod.rs:1924][WARN] Block entity at (1371,53,-484) missing id tag: NamedTag("", Compound({"y": Int(53), "Sign": String(""), "x": Int(1371), "z": Int(-484)}))

fn handle_window_event(
    game: &Arc<Game>,
    window: &winit::window::Window,
    ui_container: &mut ui::Container,
    event: winit::event::Event<InterUiMessage>,
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
            let ctx = ctx();
            let mut input = ctx.input_ctx.write();
            let (rx, ry) = if xrel > 1000.0 || yrel > 1000.0 {
                // Heuristic for if we were passed an absolute value instead of relative
                // Workaround https://github.com/tomaka/glutin/issues/1084 MouseMotion event returns absolute instead of relative values, when running Linux in a VM
                // Note SDL2 had a hint to handle this scenario:
                // sdl2::hint::set_with_priority("SDL_MOUSE_RELATIVE_MODE_WARP", "1", &sdl2::hint::Hint::Override);
                let s = 8000.0 + 0.01;
                let mouse_sens = game.settings.get_float(crate::FloatSetting::MouseSense);
                (
                    ((xrel - input.last_mouse_xrel) / s) * mouse_sens,
                    ((yrel - input.last_mouse_yrel) / s) * mouse_sens,
                )
            } else {
                let s = 2000.0 + 0.01;
                let mouse_sens = game.settings.get_float(crate::FloatSetting::MouseSense);
                ((xrel / s) * mouse_sens, (yrel / s) * mouse_sens)
            };

            input.last_mouse_xrel = xrel;
            input.last_mouse_yrel = yrel;

            use std::f64::consts::PI;

            if input.focused {
                if let Some(server) = ctx.server_ctx.load().as_ref() {
                let server = server.server.load();
                let server = server.as_ref().unwrap();
                if !server.dead.load(Ordering::Acquire) {
                if let Some(player) = server.player.load().as_ref() {
                    let mut entities = server.entities.write();
                    let mut player = entities.world.entity_mut(player.1);
                    let mut rotation = player.get_mut::<Rotation>().unwrap();
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
            }
            }
        }
        Event::UserEvent(ui_msg) => {
            match ui_msg {
                InterUiMessage::Disconnected { reason } => {
                    let ctx = ctx();
                    ctx.screen_sys.close_closable_screens();
                    ctx.screen_sys
                        .replace_screen(Box::new(ServerList::new(reason)));
                    
                    ctx.renderer.reset();
                },
            }
        }

        Event::WindowEvent { event, .. } => {
            match event {
                WindowEvent::ModifiersChanged(modifiers_state) => {
                    let mut input = ctx().input_ctx.write();
                    input.is_ctrl_pressed = modifiers_state.lcontrol_state()
                        == ModifiersKeyState::Pressed
                        || modifiers_state.rcontrol_state() == ModifiersKeyState::Pressed;
                    input.is_logo_pressed = modifiers_state.lsuper_state()
                        == ModifiersKeyState::Pressed
                        || modifiers_state.rsuper_state() == ModifiersKeyState::Pressed;
                }
                WindowEvent::CloseRequested => game.should_close.store(true, Ordering::Release),
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    ctx().input_ctx.write().dpi_factor = scale_factor;
                }

                WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                    (ElementState::Released, MouseButton::Left) => {
                        let ctx = ctx();
                        let physical_size = window.inner_size();
                        let (width, height) =
                            physical_size.to_logical::<f64>(ctx.input_ctx.read().dpi_factor).into();
                        let input = ctx.input_ctx.read();
                        if !ctx.screen_sys.clone().is_current_ingame() && !input.focused {
                            // TODO: after Pointer Lock https://github.com/rust-windowing/winit/issues/1674
                            ui_container.click_at(
                                game,
                                ctx.input_ctx.read().last_mouse_x,
                                ctx.input_ctx.read().last_mouse_y,
                                width,
                                height,
                            );
                        }
                        if let Some(server) = &ctx.server_ctx.load().as_ref() {
                            server.server.load().unwrap().on_release_left_click(input.focused);
                        }
                    }
                    (ElementState::Pressed, MouseButton::Left) => {
                        let ctx = ctx();
                        if let Some(server) = ctx.server_ctx.load().map(|server| server.server.load().as_ref()).flatten() {
                            server.on_left_click(ctx.input_ctx.read().focused);
                        }
                    }
                    (ElementState::Released, MouseButton::Right) => {
                        let ctx = ctx();
                        if let Some(server) = ctx.server_ctx.load().map(|server| server.server.load().as_ref()).flatten() {
                            server.on_release_right_click(ctx.input_ctx.read().focused);
                        }
                    }
                    (ElementState::Pressed, MouseButton::Right) => {
                        let ctx = ctx();
                        if let Some(server) = ctx.server_ctx.load().map(|server| server.server.load().as_ref()).flatten() {
                            server.on_right_click(ctx.input_ctx.read().focused);
                        }
                    }
                    (_, _) => (),
                },
                WindowEvent::CursorMoved { position, .. } => {
                    let mut input = ctx().input_ctx.write();
                    let (x, y) = position.to_logical::<f64>(input.dpi_factor).into();
                    input.last_mouse_x = x;
                    input.last_mouse_y = y;

                    if !input.focused {
                        let physical_size = window.inner_size();
                        let (width, height) =
                            physical_size.to_logical::<f64>(input.dpi_factor).into();
                        drop(input);
                        ui_container.hover_at(game, x, y, width, height);
                        if let Some(server) = game.server.load().as_ref() {
                            server.on_cursor_moved(x, y);
                        }
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    // TODO: line vs pixel delta? does pixel scrolling (e.g. touchpad) need scaling?
                    let ctx = ctx();
                    match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            ctx.screen_sys.on_scroll(x.into(), y.into());
                        }
                        MouseScrollDelta::PixelDelta(position) => {
                            let (x, y) = position.into();
                            ctx.screen_sys.on_scroll(x, y);
                        }
                    }
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    let ctx = ctx();

                    const SEMICOLON: Key = Key::Character(SmolStr::new_inline(";"));
                    if event.state == ElementState::Pressed && event.logical_key == SEMICOLON {
                        // game.console.lock().toggle(); // FIXME: support this!
                    } else {
                        let mut input = ctx.input_ctx.write();
                        match (event.state, event.logical_key) {
                            (ElementState::Pressed, Key::Named(NamedKey::F11)) => {
                                if !input.is_fullscreen {
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

                                input.is_fullscreen = !input.is_fullscreen;
                            }
                            (ElementState::Pressed, key) => {
                                #[cfg(target_os = "macos")]
                                if input.is_logo_pressed && key.eq_ignore_case('q') {
                                    input.should_close = true;
                                }
                                if !input.focused {
                                    let ctrl_pressed = input.is_ctrl_pressed || input.is_logo_pressed;
                                    ui_container.key_press(game, key.clone(), true, ctrl_pressed);
                                }
                                ctx.screen_sys.clone().press_key(
                                    (key, event.physical_key),
                                    true,
                                    game,
                                );
                            }
                            (ElementState::Released, key) => {
                                if !input.focused {
                                    let ctrl_pressed = input.is_ctrl_pressed;
                                    ui_container.key_press(game, key.clone(), false, ctrl_pressed);
                                }
                                ctx.screen_sys.clone().press_key(
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

pub(crate) fn compute_render_list(world: &World, renderer: &Arc<render::Renderer>) {
    let start_rec = Instant::now();
    // self.render_list.clone().write().clear(); // TODO: Sync with the main thread somehow!
    // renderer.clone().read()

    let mut valid_dirs = [false; 6];
    for dir in Direction::all() {
        let (ox, oy, oz) = dir.get_offset();
        let dir_vec = cgmath::Vector3::new(ox as f32, oy as f32, oz as f32);
        valid_dirs[dir.index()] = renderer.view_vector.lock().dot(dir_vec) > -0.9;
    }

    let camera = renderer.camera.lock();
    let start = (
        ((camera.pos.x as i32) >> 4),
        ((camera.pos.y as i32) >> 4),
        ((camera.pos.z as i32) >> 4),
    );
    drop(camera);

    let render_queue = Arc::new(RwLock::new(Vec::new()));
    let mut process_queue = VecDeque::with_capacity(world.chunks.read().len() * 16);
    // debug!("processqueue size {}", self.chunks.len() * 16);
    process_queue.push_front((Direction::Invalid, start));
    let _diff = Instant::now().duration_since(start_rec);
    let frustum = *renderer.frustum.lock();
    let frame_id = renderer.frame_id.load(Ordering::Acquire);
    do_render_queue(
        world,
        Arc::new(RwLock::new(process_queue)),
        frustum,
        frame_id,
        valid_dirs,
        render_queue.clone(),
    );
    let server = ctx().server_ctx.load();
    let render_list_write = &server.as_ref().unwrap().render_list;
    let mut render_list_write = render_list_write.write();
    render_list_write.clear();
    render_list_write.extend(render_queue.read().iter());
    // TODO: Improve the performance of the following by moving this to another thread!
    /*
    process_queue.par_iter().for_each(|(from, pos)| {
        let (exists, cull) = if let Some((sec, rendered_on)) =
        self.get_render_section_mut(pos.0, pos.1, pos.2)
        {
            if rendered_on == renderer.frame_id {
                return;
            }
            if let Some(chunk) = self.chunks.clone().write().get_mut(&CPos(pos.0, pos.2)) {
                chunk.sections_rendered_on[pos.1 as usize] = renderer.frame_id;
            }

            let min = cgmath::Point3::new(
                pos.0 as f32 * 16.0,
                -pos.1 as f32 * 16.0,
                pos.2 as f32 * 16.0,
            );
            let bounds =
                collision::Aabb3::new(min, min + cgmath::Vector3::new(16.0, -16.0, 16.0));
            if renderer.frustum.contains(&bounds) == collision::Relation::Out
                && *from != Direction::Invalid
            {
                return;
            }
            (
                sec.is_some(),
                sec.map_or(chunk_builder::CullInfo::all_vis(), |v| v.clone().read().cull_info),
            )
        } else {
            return;
        };

        if exists {
            self.render_list.clone().write().push(*pos);
        }

        for dir in Direction::all() {
            let (ox, oy, oz) = dir.get_offset();
            let opos = (pos.0 + ox, pos.1 + oy, pos.2 + oz);
            if let Some((_, rendered_on)) = self.get_render_section_mut(opos.0, opos.1, opos.2)
            {
                if rendered_on == renderer.frame_id {
                    continue;
                }
                if *from == Direction::Invalid
                    || (valid_dirs[dir.index()] && cull.is_visible(*from, dir))
                {
                    process_queue.push_back((dir.opposite(), opos));
                }
            }
        }
    });*/

    /*while let Some((from, pos)) = process_queue.pop_front() { // TODO: Use par iters
        let (exists, cull) = if let Some((sec, rendered_on)) =
            self.get_render_section_mut(pos.0, pos.1, pos.2)
        {
            if rendered_on == renderer.frame_id {
                continue;
            }
            if let Some(chunk) = self.chunks.clone().write().get_mut(&CPos(pos.0, pos.2)) {
                chunk.sections_rendered_on[pos.1 as usize] = renderer.frame_id;
            }

            let min = cgmath::Point3::new(
                pos.0 as f32 * 16.0,
                -pos.1 as f32 * 16.0,
                pos.2 as f32 * 16.0,
            );
            let bounds =
                collision::Aabb3::new(min, min + cgmath::Vector3::new(16.0, -16.0, 16.0));
            if renderer.frustum.contains(&bounds) == collision::Relation::Out
                && from != Direction::Invalid
            {
                continue;
            }
            (
                sec.is_some(),
                sec.map_or(chunk_builder::CullInfo::all_vis(), |v| v.clone().read().cull_info),
            )
        } else {
            continue;
        };

        if exists {
            self.render_list.clone().write().push(pos);
        }

        for dir in Direction::all() {
            let (ox, oy, oz) = dir.get_offset();
            let opos = (pos.0 + ox, pos.1 + oy, pos.2 + oz);
            if let Some((_, rendered_on)) = self.get_render_section_mut(opos.0, opos.1, opos.2)
            {
                if rendered_on == renderer.frame_id {
                    continue;
                }
                if from == Direction::Invalid
                    || (valid_dirs[dir.index()] && cull.is_visible(from, dir))
                {
                    process_queue.push_back((dir.opposite(), opos));
                }
            }
        }
    }*/
}

#[allow(clippy::type_complexity)]
pub(crate) fn do_render_queue(
    world: &World,
    process_queue: Arc<RwLock<VecDeque<(Direction, (i32, i32, i32))>>>,
    frustum: Frustum<f32>,
    frame_id: u64,
    valid_dirs: [bool; 6],
    render_queue: Arc<RwLock<Vec<(i32, i32, i32)>>>,
) {
    let out = Arc::new(RwLock::new(VecDeque::new()));
    /*let tmp_renderer = renderer.clone();
    let tmp_renderer = tmp_renderer.read();
    let frame_id = tmp_renderer.frame_id.clone();*/
    // let frame_id = renderer.clone().read().frame_id.clone();
    // let frustum = renderer.clone().read().frustum.clone().read().as_ref().unwrap();
    let tmp_frustum = frustum;
    // debug!("rendering {} elems", process_queue.clone().read().len());
    process_queue.read().iter().for_each(|(from, pos)| {
        let (exists, cull) = if let Some((sec, rendered_on)) =
            get_render_section_mut(world, pos.0, pos.1, pos.2)
        {
            if rendered_on == frame_id {
                return;
            }
            if let Some(chunk) = world.chunks.write().get_mut(&CPos(pos.0, pos.2)) {
                chunk.sections_rendered_on[pos.1 as usize] = frame_id;
            }

            let min = cgmath::Point3::new(
                pos.0 as f32 * 16.0,
                -pos.1 as f32 * 16.0,
                pos.2 as f32 * 16.0,
            );
            let bounds =
                collision::Aabb3::new(min, min + cgmath::Vector3::new(16.0, -16.0, 16.0));
            if tmp_frustum.contains(&bounds) == collision::Relation::Out
                && *from != Direction::Invalid
            {
                return;
            }
            (
                sec.is_some(),
                sec.map_or(chunk_builder::CullInfo::all_vis(), |v| v),
            )
        } else {
            return;
        };

        if exists {
            render_queue.clone().write().push(*pos);
        }

        for dir in Direction::all() {
            let (ox, oy, oz) = dir.get_offset();
            let opos = (pos.0 + ox, pos.1 + oy, pos.2 + oz);
            if let Some((_, rendered_on)) = get_render_section_mut(world, opos.0, opos.1, opos.2)
            {
                if rendered_on == frame_id {
                    continue;
                }
                if *from == Direction::Invalid
                    || (valid_dirs[dir.index()] && cull.is_visible(*from, dir))
                {
                    out.clone().write().push_back((dir.opposite(), opos));
                }
            }
        }
    });
    if !out.read().is_empty() {
        do_render_queue(world, out, frustum, frame_id, valid_dirs, render_queue);
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn get_render_list(world: &World) -> Vec<((i32, i32, i32), Arc<RwLock<render::ChunkBuffer>>)> {
    ctx().server_ctx.load().as_ref().unwrap().render_list
        .clone()
        .read()
        .iter()
        // .par_iter()
        .filter_map(|v| {
            let chunks = world.chunks.read();
            let chunk = chunks.get(&CPos(v.0, v.2));
            if let Some(chunk) = chunk {
                if let Some(sec) = chunk.sections[v.1 as usize].as_ref() {
                    return Some((*v, sec.render_buffer.clone()));
                }
            }
            None
        })
        .collect()
}

/*
pub fn get_section_mut(&self, x: i32, y: i32, z: i32) -> Option<Section> {
    if let Some(chunk) = self.chunks.clone().get(&CPos(x, z)) {
        if let Some(sec) = chunk.sections[y as usize].as_ref() {
            return Some(sec.clone());
        }
    }
    None
}*/

// TODO: Improve the perf of this method as it is the MAIN bottleneck slowing down the program!
fn get_render_section_mut(world: &World, x: i32, y: i32, z: i32) -> Option<(Option<CullInfo>, u64)> {
    if !(0..=15).contains(&y) {
        return None;
    }
    if let Some(chunk) = world.chunks.read().get(&CPos(x, z)) {
        let rendered = &chunk.sections_rendered_on[y as usize];
        if let Some(sec) = chunk.sections[y as usize].as_ref() {
            return Some((Some(sec.cull_info), *rendered));
        }
        return Some((None, *rendered));
    }
    None
}

fn spawn_render_list_computer(
    world: Arc<World>,
    renderer: Arc<Renderer>,
) -> (Sender<bool>, Receiver<bool>) {
    let (tx, rx) = unbounded();
    let (etx, erx) = unbounded();
    thread::spawn(move || loop {
        let _ = rx.recv().unwrap();
        compute_render_list(&world, &renderer);
        while rx.try_recv().is_ok() {}
        etx.send(true).unwrap();
    });
    (tx, erx)
}

#[derive(Resource)]
pub struct RendererResource(pub Arc<Renderer>);

#[derive(Resource)]
pub struct ScreenSystemResource(pub Arc<ScreenSystem>);

#[derive(Resource)]
pub struct InventoryContextResource(pub Arc<RwLock<InventoryContext>>);
