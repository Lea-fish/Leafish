use crate::inventory::{Inventory, InventoryType, Item, Material, Slot};
use crate::render::hud::{Hud, HudContext};
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use std::sync::Arc;

use leafish_protocol::protocol::Version;
use parking_lot::RwLock;
use std::sync::atomic::Ordering;
use crate::inventory::player_inventory::PlayerInventory;

pub struct BaseInventory {
    dirty: bool,
    x_offset: f64,
    y_offset: f64,
    custom_offset: bool,
    hud_context: Arc<RwLock<HudContext>>,
    player_inventory: Arc<RwLock<PlayerInventory>>,
}

impl BaseInventory {
    pub fn new(
        hud_context: Arc<RwLock<HudContext>>,
        player_inventory: Arc<RwLock<PlayerInventory>>,
        renderer: &Renderer,
    ) -> Self {
        let icon_scale = Hud::icon_scale(renderer);
        let size = 16.0;
        let hot_bar_offset = 6.0;
        let slot_offset = size + size * 1.0 / 8.0;
        Self {
            dirty: false,
            x_offset: -4.5,
            y_offset: ((renderer.safe_height as f64 / icon_scale + 166.0) / 2.0 - slot_offset - hot_bar_offset) / 16.0,
            custom_offset: false,
            hud_context,
            player_inventory,
        }
    }

    fn update_icons(&mut self, renderer: &Renderer) {
        let scale = Hud::icon_scale(renderer);
        let size = scale * 16.0;
        println!("base x offset {}", self.x_offset);
        println!("base y offset {}", self.y_offset);
        let x_offset = size * self.x_offset;
        let y_offset = size * self.y_offset;
        let hot_bar_offset = scale * 4.0;
        let slot_offset = size + size * 1.0 / 8.0;
        for y in (0..3).rev() {
            for x in 0..9 {
                self.player_inventory.clone().write()
                    .get_raw_slot_mut(9 + x + 9 * (2 - y))
                    .update_position(
                        x_offset + x as f64 * slot_offset,
                        y_offset + -((y as f64) * slot_offset + hot_bar_offset + slot_offset),
                        size,
                    );
                println!("inv {} | y: {}: {}, {}", 9 * (2 - y) + x, y, x_offset + x as f64 * slot_offset, y_offset + -((y as f64) * slot_offset + hot_bar_offset/* + slot_offset*/)/* + -(y as f64 * slot_offset * 2.0 + hot_bar_offset)*/);
            }
        }
        for i in 0..9 {
            self.player_inventory.clone().write().get_raw_slot_mut(36 + i).update_position(
                x_offset + i as f64 * (size + size * 1.0 / 8.0),
                y_offset,
                size,
            );
           println!("hotbar {}: {}, {}", i, x_offset + i as f64 * (size + size * 1.0 / 8.0), y_offset);
        }
        self.dirty = true;
    }

    pub fn update_offset(&mut self, x_offset: f64, y_offset: f64, renderer: &Renderer) {
        self.x_offset = x_offset;
        self.y_offset = y_offset;
        self.custom_offset = true;
        self.update_icons(renderer);
    }
}

impl Inventory for BaseInventory {
    fn size(&self) -> u16 {
        36
    }

    fn id(&self) -> i32 {
        -1
    }

    fn name(&self) -> Option<&String> {
        None
    }

    fn get_item(&self, slot: u16) -> Option<Item> {
        self.player_inventory.clone().write().get_raw_slot_mut(9 + slot).item.clone()
    }

    fn set_item(&mut self, slot: u16, item: Option<Item>) {
        self.player_inventory.clone().write().set_item(9 + slot, item);
        self.dirty = true;
    }

    fn init(
        &mut self,
        renderer: &mut Renderer,
        _ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        inventory_window.elements.push(vec![]);
        if !self.custom_offset {
            let icon_scale = Hud::icon_scale(renderer);
            let size = 16.0;
            let hot_bar_offset = 6.0;
            let slot_offset = size + size * 1.0 / 8.0;
            self.y_offset = ((renderer.safe_height as f64 / icon_scale + 166.0) / 2.0 - slot_offset - hot_bar_offset) / 16.0;
        }
        self.update_icons(renderer);
        self.hud_context
            .clone()
            .read()
            .dirty_slots
            .store(true, Ordering::Relaxed);
    }

    fn tick(
        &mut self,
        renderer: &mut Renderer,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        if self.dirty {
            self.dirty = false;
            inventory_window.elements.get_mut(0).unwrap().clear();
            for slot in 9..45 {
                let player_inventory = self.player_inventory.clone();
                let mut player_inventory = player_inventory.write();
                let slot = player_inventory.get_raw_slot_mut(slot);
                if slot.item.is_some() {
                    inventory_window.draw_item_internally(
                        slot.item.as_ref().unwrap(),
                        slot.x,
                        slot.y,
                        0,
                        ui_container,
                        renderer,
                        VAttach::Top,
                    );
                }
            }
        }
    }

    fn close(&mut self) {
        // TODO
    }

    fn click_at(&self, _cursor: (u32, u32)) {
        // TODO
    }

    fn resize(
        &mut self,
        _width: u32,
        _height: u32,
        renderer: &mut Renderer,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        self.init(renderer, ui_container, inventory_window);
    }

    fn ty(&self) -> InventoryType {
        InventoryType::Base
    }
}
