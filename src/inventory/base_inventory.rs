use crate::inventory::{Inventory, InventoryType, Item};
use crate::render::hud::{Hud, HudContext};
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui::{Container, VAttach};
use std::sync::Arc;

use crate::inventory::Slot;
use parking_lot::RwLock;
use std::sync::atomic::Ordering;

pub struct BaseInventory {
    slots: Vec<Slot>,
    dirty: bool,
    x_offset: f64,
    y_offset: f64,
    hud_context: Arc<RwLock<HudContext>>,
}

impl BaseInventory {
    pub fn new(hud_context: Arc<RwLock<HudContext>>, renderer: Arc<Renderer>) -> Self {
        let mut slots = vec![];
        for _ in (0..3).rev() {
            for _ in 0..9 {
                slots.push(Slot::new(0.0, 0.0, 0.0));
            }
        }
        for _ in 0..9 {
            slots.push(Slot::new(0.0, 0.0, 0.0));
        }
        let mut inv = Self {
            slots,
            dirty: false,
            x_offset: 0.0,
            y_offset: 0.0,
            hud_context,
        };
        inv.update_icons(renderer);
        inv
    }

    fn update_icons(&mut self, renderer: Arc<Renderer>) {
        let scale = Hud::icon_scale(renderer);
        let size = scale * 16.0;
        let x_offset = size * self.x_offset;
        let y_offset = size * self.y_offset;
        let hot_bar_offset = scale * 4.0;
        let slot_offset = size + size / 8.0;
        for y in (0..3).rev() {
            for x in 0..9 {
                self.slots[x + y * 9].update_position(
                    x_offset + x as f64 * slot_offset,
                    y_offset + -((y as f64) * slot_offset + hot_bar_offset + slot_offset),
                    size,
                );
            }
        }
        for i in 0..9 {
            self.slots[27 + i].update_position(x_offset + i as f64 * slot_offset, y_offset, size);
        }
        self.dirty = true;
    }

    pub fn update_offset(&mut self, x_offset: f64, y_offset: f64, renderer: Arc<Renderer>) {
        self.x_offset = x_offset;
        self.y_offset = y_offset;
        self.update_icons(renderer);
    }
}

impl Inventory for BaseInventory {
    fn size(&self) -> u16 {
        36
    }

    fn id(&self) -> i32 {
        panic!("Base inventory doesn't have an id");
    }

    fn get_client_state_id(&self) -> i16 {
        panic!("Base inventory doesn't have a state id number");
    }

    fn set_client_state_id(&mut self, _client_state_id: i16) {
        panic!("Base inventory doesn't have a state id number");
    }

    fn get_item(&self, slot_id: u16) -> Option<Item> {
        self.slots[slot_id as usize].item.clone()
    }

    fn set_item(&mut self, slot_id: u16, item: Option<Item>) {
        self.slots[slot_id as usize].item = item;
        self.dirty = true;
        self.hud_context
            .clone()
            .read()
            .dirty_slots
            .store(true, Ordering::Relaxed);
    }

    fn init(
        &mut self,
        renderer: Arc<Renderer>,
        _ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        inventory_window.elements.push(vec![]);
        self.update_icons(renderer);
        self.hud_context
            .clone()
            .read()
            .dirty_slots
            .store(true, Ordering::Relaxed);
    }

    fn tick(
        &mut self,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        if self.dirty {
            self.dirty = false;
            inventory_window.elements.get_mut(0).unwrap().clear();
            for slot in &self.slots {
                if slot.item.is_some() {
                    inventory_window.draw_item_internally(
                        slot.item.as_ref().unwrap(),
                        slot.x,
                        slot.y,
                        0,
                        ui_container,
                        renderer.clone(),
                        VAttach::Top,
                    );
                }
            }
        }
    }

    fn get_slot(&self, x: f64, y: f64) -> Option<u8> {
        for (i, slot) in self.slots.iter().enumerate() {
            if slot.is_within(x, y) {
                return Some(i as u8);
            }
        }
        None
    }

    fn resize(
        &mut self,
        _width: u32,
        _height: u32,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        self.init(renderer, ui_container, inventory_window);
    }

    fn ty(&self) -> InventoryType {
        InventoryType::Internal
    }
}
