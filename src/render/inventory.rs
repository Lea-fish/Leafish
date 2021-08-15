use crate::ui::{ImageRef, Text, Container};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use crate::inventory::{InventoryContext, Inventory};
use crate::screen::Screen;
use crate::render::Renderer;

pub struct InventoryWindow {

    pub elements: Vec<Vec<ImageRef>>,
    pub inventory: Arc<RwLock<dyn Inventory + Sync + Send>>

}

impl Screen for InventoryWindow {

    fn on_active(&mut self, renderer: &mut Renderer, ui_container: &mut Container) {
        self.inventory.clone().write().unwrap().init(renderer, ui_container, self);
    }

    fn on_deactive(&mut self, _renderer: &mut Renderer, _ui_container: &mut Container) {
        self.inventory.clone().write().unwrap().close();
        for mut element in &mut self.elements {
            element.clear();
        }
        self.elements.clear();
    }

    fn tick(&mut self, _delta: f64, renderer: &mut Renderer, ui_container: &mut Container) -> Option<Box<dyn Screen>> {
        self.inventory.clone().write().unwrap().tick(renderer, ui_container, self);
        None
    }

    fn is_closable(&self) -> bool {
        true
    }

}