use crate::inventory::{Inventory, InventoryContext, Item};
use crate::render::hud::Hud;
use crate::render::Renderer;
use crate::screen::Screen;
use crate::ui;
use crate::ui::{Container, ImageRef, TextRef};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct InventoryWindow {
    pub elements: Vec<Vec<ImageRef>>,
    pub text_elements: Vec<Vec<TextRef>>,
    pub inventory: Arc<RwLock<dyn Inventory + Sync + Send>>,
    inventory_context: Arc<RwLock<InventoryContext>>,
}

impl Screen for InventoryWindow {
    fn on_active(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        self.inventory_context
            .clone()
            .write()
            .inventory
            .replace(self.inventory.clone());
        self.inventory
            .clone()
            .write()
            .init(renderer, ui_container, self);
    }

    fn on_deactive(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {
        self.inventory_context.clone().write().inventory = None;
        self.inventory.clone().write().close(self);
        self.clear_elements();
    }

    fn tick(
        &mut self,
        _delta: f64,
        renderer: &mut Renderer,
        ui_container: &mut Container,
    ) -> Option<Box<dyn Screen>> {
        self.inventory
            .clone()
            .write()
            .tick(renderer, ui_container, self);
        None
    }

    fn on_resize(
        &mut self,
        width: u32,
        height: u32,
        renderer: &mut Renderer,
        ui_container: &mut Container,
    ) {
        self.inventory
            .clone()
            .write()
            .resize(width, height, renderer, ui_container, self);
    }

    fn is_closable(&self) -> bool {
        true
    }
}

impl InventoryWindow {
    pub fn new(
        inventory: Arc<RwLock<dyn Inventory + Sync + Send>>,
        inventory_context: Arc<RwLock<InventoryContext>>,
    ) -> Self {
        InventoryWindow {
            elements: vec![],
            text_elements: vec![],
            inventory,
            inventory_context,
        }
    }
}

impl InventoryWindow {
    pub fn draw_item(
        &mut self,
        item: &Item,
        x: f64,
        y: f64,
        elements_idx: usize,
        ui_container: &mut Container,
        renderer: &Renderer,
    ) {
        let icon_scale = Hud::icon_scale(renderer);
        let textures = item.material.texture_locations();
        let texture = if Renderer::get_texture_optional(&renderer.textures, &*textures.0).is_some()
        {
            textures.0
        } else {
            textures.1
        };
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0, 0.0, 1.0, 1.0))
            .position(x, y)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .size(icon_scale * 16.0, icon_scale * 16.0)
            .texture(format!("minecraft:{}", texture))
            .create(ui_container);
        self.elements.get_mut(elements_idx).unwrap().push(image);
    }

    pub fn clear_elements(&mut self) {
        for element in &mut self.elements {
            element.clear();
        }
        self.elements.clear();
        for element in &mut self.text_elements {
            element.clear();
        }
        self.text_elements.clear();
    }
}
