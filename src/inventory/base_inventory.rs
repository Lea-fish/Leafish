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
    hud_context: Arc<RwLock<HudContext>>,
    player_inventory: Arc<RwLock<PlayerInventory>>,
}

impl BaseInventory {
    pub fn new(
        hud_context: Arc<RwLock<HudContext>>,
        player_inventory: Arc<RwLock<PlayerInventory>>,
    ) -> Self {
        Self {
            dirty: false,
            hud_context,
            player_inventory,
        }
    }

    fn update_icons(&mut self, renderer: &Renderer) {
        let scale = Hud::icon_scale(renderer);
        let size = scale * 16.0;
        let x_offset = -(size * 4.5);
        let y_offset = size * 4.18;
        let hot_bar_offset = scale * 4.0;
        let slot_offset = size + size * 1.0 / 8.0;
        for y in (0..3).rev() {
            for x in 0..9 {
                self.player_inventory.clone().write()
                    .get_raw_slot_mut(x + 9 * (2 - y))
                    .update_position(
                        x_offset + x as f64 * slot_offset,
                        y_offset + -(y as f64 * slot_offset * 2.0 + hot_bar_offset),
                        size,
                    );
            }
        }
        for i in 0..9 {
            self.player_inventory.clone().write().get_raw_slot_mut(27 + i).update_position(
                x_offset + i as f64 * (size + size * 1.0 / 8.0),
                y_offset,
                size,
            );
        }
        self.dirty = true;
    }
}

impl Inventory for BaseInventory {
    fn size(&self) -> u16 {
        36
    }

    fn id(&self) -> i8 {
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
    }

    fn init(
        &mut self,
        renderer: &mut Renderer,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        inventory_window.elements.push(vec![]);
        let basic_elements = inventory_window.elements.get_mut(0).unwrap();
        let icon_scale = Hud::icon_scale(renderer);
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 256.0, 0.0 / 256.0, 176.0 / 256.0, 166.0 / 256.0))
            .position(0.0, 0.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .size(icon_scale * 176.0, icon_scale * 166.0)
            .texture("minecraft:gui/container/inventory")
            .create(ui_container);
        basic_elements.push(image);
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
        renderer: &mut Renderer,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        if self.dirty {
            self.dirty = false;
            inventory_window.elements.get_mut(1).unwrap().clear();
            for slot in 9..45 {
                let player_inventory = self.player_inventory.clone();
                let mut player_inventory = player_inventory.write();
                let slot = player_inventory.get_raw_slot_mut(slot);
                if slot.item.is_some() {
                    inventory_window.draw_item(
                        slot.item.as_ref().unwrap(),
                        slot.x,
                        slot.y,
                        1,
                        ui_container,
                        renderer,
                    );
                }
            }
        }
    }

    fn close(&mut self, _inventory_window: &mut InventoryWindow) {
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
        inventory_window.clear_elements();
        self.init(renderer, ui_container, inventory_window);
    }

    fn ty(&self) -> InventoryType {
        InventoryType::Base
    }
}
