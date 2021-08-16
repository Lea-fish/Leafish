use crate::ui::{ImageRef, Text, Container, TextRef};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use crate::inventory::{InventoryContext, Inventory, Item, Material};
use crate::screen::Screen;
use crate::render::Renderer;
use crate::{gl, ui};
use crate::render::hud::Hud;

pub struct InventoryWindow {

    pub elements: Vec<Vec<ImageRef>>,
    pub text_elements: Vec<Vec<TextRef>>,
    pub inventory: Arc<RwLock<dyn Inventory + Sync + Send>>,
    inventory_context: Arc<RwLock<InventoryContext>>,

}

impl Screen for InventoryWindow {

    fn on_active(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        self.inventory_context.clone().write().unwrap().inventory.replace(self.inventory.clone());
        self.inventory.clone().write().unwrap().init(renderer, ui_container, self);
    }

    fn on_deactive(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {
        self.inventory_context.clone().write().unwrap().inventory = None;
        self.inventory.clone().write().unwrap().close(self);
        self.clear_elements();
    }

    fn tick(&mut self, _delta: f64, renderer: &mut Renderer, ui_container: &mut Container) -> Option<Box<dyn Screen>> {
        self.inventory.clone().write().unwrap().tick(renderer, ui_container, self);
        None
    }

    fn on_resize(&mut self, width: u32, height: u32, renderer: &mut Renderer, ui_container: &mut Container) {
        self.inventory.clone().write().unwrap().resize(width, height, renderer, ui_container, self);
    }

    fn is_closable(&self) -> bool {
        true
    }

}

impl InventoryWindow {
    
    pub fn new(inventory: Arc<RwLock<dyn Inventory + Sync + Send>>, inventory_context: Arc<RwLock<InventoryContext>>) -> Self {
        InventoryWindow {
            elements: vec![],
            text_elements: vec![],
            inventory,
            inventory_context
        }
    }
    
}

impl InventoryWindow {

    pub fn draw_item(&mut self, item: &Item, x: f64, y: f64, elements_idx: usize,
                     ui_container: &mut Container, renderer: &Renderer) {
        let icon_scale = Hud::icon_scale(renderer);
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 16.0, 0.0 / 16.0, 16.0 / 16.0, 16.0 / 16.0))
            .position(x, y)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .size(icon_scale as f64 / 9.0 * 16.0, icon_scale as f64 / 9.0 * 16.0)
            .texture(format!("minecraft:{}", item.material.texture_location()))
            .create(ui_container);
        self.elements.get_mut(elements_idx).unwrap().push(image);
    }

    pub fn clear_elements(&mut self) {
        for mut element in &mut self.elements {
            element.clear();
        }
        self.elements.clear();
        for mut element in &mut self.text_elements {
            element.clear();
        }
        self.text_elements.clear();
    }

}