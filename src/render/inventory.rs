use crate::inventory::{Inventory, InventoryContext, Item};
use crate::render::hud::Hud;
use crate::render::Renderer;
use crate::screen::Screen;
use crate::{ui, Game};
use crate::ui::{Container, ImageRef, TextRef, VAttach};
use parking_lot::RwLock;
use std::sync::Arc;
use crate::inventory::base_inventory::BaseInventory;
use glutin::event::VirtualKeyCode;

#[derive(Clone)]
pub struct InventoryWindow {
    pub elements: Vec<Vec<ImageRef>>,
    pub text_elements: Vec<Vec<TextRef>>,
    pub inventory: Arc<RwLock<dyn Inventory + Sync + Send>>,
    base_inventory: Arc<RwLock<BaseInventory>>,
    inventory_context: Arc<RwLock<InventoryContext>>,
}

impl Screen for InventoryWindow {
    fn init(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        self.inventory_context
            .clone()
            .write()
            .inventory
            .replace(self.inventory.clone());
        self.base_inventory
            .clone()
            .write()
            .init(renderer, ui_container, self);
        self.inventory
            .clone()
            .write()
            .init(renderer, ui_container, self);
    }

    fn deinit(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {
        self.inventory_context.clone().write().inventory = None;
        self.base_inventory.clone().write().close();
        self.inventory.clone().write().close();
        self.clear_elements();
    }

    fn on_active(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {}

    fn on_deactive(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {}

    fn tick(
        &mut self,
        _delta: f64,
        renderer: &mut Renderer,
        ui_container: &mut Container,
    ) -> Option<Box<dyn Screen>> {
        self.base_inventory
            .clone()
            .write()
            .tick(renderer, ui_container, self);
        self.inventory
            .clone()
            .write()
            .tick(renderer, ui_container, self);
        None
    }

    fn on_resize(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        self.clear_elements();
        self.base_inventory.clone().write().resize(
            renderer.safe_width,
            renderer.safe_height,
            renderer,
            ui_container,
            self,
        );
        self.inventory.clone().write().resize(
            renderer.safe_width,
            renderer.safe_height,
            renderer,
            ui_container,
            self,
        );
    }

    fn is_closable(&self) -> bool {
        true
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }

    fn on_key_press(&mut self, key: VirtualKeyCode, down: bool, game: &mut Game) -> bool {
        if key == VirtualKeyCode::Escape && !down {
            self.inventory_context.clone().write().try_close_inventory(game.screen_sys.clone());
            return true;
        }
        false
    }
}

impl InventoryWindow {
    pub fn new(
        inventory: Arc<RwLock<dyn Inventory + Sync + Send>>,
        inventory_context: Arc<RwLock<InventoryContext>>,
        base_inventory: Arc<RwLock<BaseInventory>>,
    ) -> Self {
        Self {
            elements: vec![],
            text_elements: vec![],
            inventory,
            base_inventory,
            inventory_context,
        }
    }
}

impl InventoryWindow {
    pub fn draw_item(
        item: &Item,
        x: f64,
        y: f64,
        elements: &mut Vec<ImageRef>,
        ui_container: &mut Container,
        renderer: &Renderer,
        v_attach: VAttach,
    ) {
        let icon_scale = Hud::icon_scale(renderer);
        let textures = item.material.texture_locations();
        let texture = if let Some(tex) = Renderer::get_texture_optional(&renderer.textures, &*textures.0)
        {
            if tex.dummy {
                textures.1
            } else {
                textures.0
            }
        } else {
            textures.1
        };
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0, 0.0, 1.0, 1.0))
            .position(x, y)
            .alignment(v_attach, ui::HAttach::Center)
            .size(icon_scale * 16.0, icon_scale * 16.0)
            .texture(format!("minecraft:{}", texture))
            .create(ui_container);
        elements.push(image);
    }

    pub fn draw_item_internally(
        &mut self,
        item: &Item,
        x: f64,
        y: f64,
        elements_idx: usize,
        ui_container: &mut Container,
        renderer: &Renderer,
        v_attach: VAttach,
    ) {
        Self::draw_item(item, x, y, self.elements.get_mut(elements_idx).unwrap(), ui_container, renderer, v_attach);
    }

    fn clear_elements(&mut self) {
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
