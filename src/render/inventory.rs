use crate::inventory::{Inventory, InventoryContext, Item};
use crate::render::hud::Hud;
use crate::render::Renderer;
use crate::screen::{Screen, ScreenSystem};
use crate::ui::{Container, ImageRef, TextBoxRef, TextRef, VAttach};
use crate::{ui, Game};
use glutin::event::VirtualKeyCode;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone)]
pub struct InventoryWindow {
    pub elements: Vec<Vec<ImageRef>>,
    pub text_elements: Vec<Vec<TextRef>>,
    pub cursor_element: Vec<ImageRef>,
    pub text_box: Vec<TextBoxRef>,
    pub inventory: Arc<RwLock<dyn Inventory + Sync + Send>>,
    pub inventory_context: Arc<RwLock<InventoryContext>>,
}

impl Screen for InventoryWindow {
    fn init(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        self.text_elements.push(vec![]); // numbers of items in inventory
        self.text_elements.push(vec![]); // number for item in child inventory
        self.text_elements.push(vec![]); // number for item under cursor
        self.inventory_context
            .write()
            .inventory
            .replace(self.inventory.clone());
        self.inventory
            .clone()
            .write()
            .init(renderer, ui_container, self);
    }

    fn deinit(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
        self.inventory_context.clone().write().inventory = None;
        self.clear_elements();
    }

    fn on_active(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
    }

    fn on_deactive(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<Renderer>,
        _ui_container: &mut Container,
    ) {
    }

    fn tick(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        _delta: f64,
    ) {
        self.inventory
            .clone()
            .write()
            .tick(renderer.clone(), ui_container, self);
        self.inventory_context
            .clone()
            .write()
            .draw_cursor(renderer, ui_container, self);
    }

    fn on_resize(
        &mut self,
        _screen_sys: &ScreenSystem,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        self.clear_elements();
        self.inventory.clone().write().resize(
            renderer.screen_data.read().safe_width,
            renderer.screen_data.read().safe_height,
            renderer.clone(),
            ui_container,
            self,
        );
    }

    fn on_key_press(&mut self, key: VirtualKeyCode, down: bool, game: &mut Game) {
        if key == VirtualKeyCode::Escape && !down {
            self.inventory_context
                .write()
                .try_close_inventory(game.screen_sys.clone());
        }
    }

    fn is_closable(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}

impl InventoryWindow {
    pub fn new(
        inventory: Arc<RwLock<dyn Inventory + Sync + Send>>,
        inventory_context: Arc<RwLock<InventoryContext>>,
    ) -> Self {
        Self {
            elements: vec![],
            text_elements: vec![],
            inventory,
            inventory_context,
            cursor_element: vec![],
            text_box: vec![],
        }
    }
}

impl InventoryWindow {
    pub fn draw_item(
        item: &Item,
        x: f64,
        y: f64,
        elements: &mut Vec<ImageRef>,
        text_elements: &mut Vec<TextRef>,
        ui_container: &mut Container,
        renderer: Arc<Renderer>,
        v_attach: VAttach,
    ) {
        let icon_scale = Hud::icon_scale(renderer.clone());
        let textures = item.material.texture_locations();
        let texture =
            if let Some(tex) = Renderer::get_texture_optional(&renderer.textures, &textures.0) {
                if tex.dummy {
                    textures.1
                } else {
                    textures.0
                }
            } else {
                textures.1
            };
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0, 0.0, 256.0, 256.0))
            .position(x, y)
            .alignment(v_attach, ui::HAttach::Left)
            .size(icon_scale * 16.0, icon_scale * 16.0)
            .texture(format!("minecraft:{}", texture))
            .create(ui_container);
        elements.push(image);

        if item.stack.count != 1 {
            let text = ui::TextBuilder::new()
                .scale_x(icon_scale / 2.0)
                .scale_y(icon_scale / 2.0)
                .text(item.stack.count.to_string())
                .position(x, y)
                .alignment(v_attach, ui::HAttach::Left)
                .colour((255, 255, 255, 255))
                .shadow(true)
                .create(ui_container);
            text_elements.push(text);
        }
    }

    pub fn draw_item_internally(
        &mut self,
        item: &Item,
        x: f64,
        y: f64,
        elements_idx: usize,
        ui_container: &mut Container,
        renderer: Arc<Renderer>,
        v_attach: VAttach,
    ) {
        Self::draw_item(
            item,
            x,
            y,
            self.elements.get_mut(elements_idx).unwrap(),
            &mut self.text_elements.get_mut(elements_idx).unwrap(),
            ui_container,
            renderer,
            v_attach,
        );
    }

    fn clear_elements(&mut self) {
        for element in &mut self.elements {
            element.clear();
        }
        self.elements.clear();
        for element in &mut self.text_elements {
            element.clear();
        }
        self.text_box.clear();
    }
}
