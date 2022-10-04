use crate::inventory::{Inventory, InventoryType, Item, Slot};
use crate::render::hud::{Hud, HudContext};
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use std::sync::Arc;

use crate::inventory::base_inventory::BaseInventory;
use parking_lot::RwLock;
use std::sync::atomic::Ordering;

pub struct ChestInventory {
    slots: Vec<Slot>,
    dirty: bool,
    hud_context: Arc<RwLock<HudContext>>,
    inv_below: Arc<RwLock<BaseInventory>>,
    name: String,
    slot_count: u16,
    id: i32,
    action_number: i16,
}

impl ChestInventory {
    pub fn new(
        renderer: Arc<Renderer>,
        hud_context: Arc<RwLock<HudContext>>,
        inv_below: Arc<RwLock<BaseInventory>>,
        slot_count: u16,
        name: String,
        id: i32,
    ) -> Self {
        let scale = Hud::icon_scale(renderer.clone());
        let size = scale * 16.0;
        let slot_offset = size + size * 1.0 / 8.0;
        let x_offset = -(size * 4.5);
        let y = 114;
        let rows = slot_count / 9;
        let y_size = y + rows * 18;
        let y_offset = (renderer.screen_data.read().safe_height as f64 / scale - y_size as f64)
            / 2.0
            + slot_offset / 2.0;
        let hot_bar_offset = scale * 4.0;
        let mut slots = vec![];
        let rows = (slot_count / 9) as usize;
        for y in (0..rows).rev() {
            for x in 0..9 {
                slots.push(Slot::new(
                    x_offset + (x as f64) * (size + size * 1.0 / 8.0),
                    y_offset
                        + -((y as f64 + 1.0 / 8.0) * (size + size * 1.0 / 8.0)
                            + hot_bar_offset / 2.0),
                    size,
                ));
            }
        }
        Self {
            slots,
            dirty: false,
            hud_context,
            inv_below,
            name,
            slot_count,
            id,
            action_number: 0,
        }
    }

    fn update_icons(&mut self, renderer: Arc<Renderer>) {
        let scale = Hud::icon_scale(renderer.clone());
        let size = scale * 16.0;
        let slot_offset = size + size * 1.0 / 8.0;
        let x_offset = -(size * 4.5);
        let icon_scale = Hud::icon_scale(renderer.clone());
        let y = 114;
        let rows = self.slot_count / 9;
        let y_size = y + rows * 18;
        let y_offset =
            (renderer.screen_data.read().safe_height as f64 / icon_scale - y_size as f64) / 2.0;
        let rows = (self.slot_count / 9) as usize;
        for y in (0..rows).rev() {
            for x in 0..9 {
                self.slots
                    .get_mut(x + 9 * ((rows - 1) - y))
                    .unwrap()
                    .update_position(
                        x_offset + x as f64 * slot_offset,
                        scale * (y_offset + (rows * 18) as f64) + -((y as f64) * slot_offset),
                        size,
                    );
            }
        }
        self.inv_below.clone().write().update_offset(
            x_offset / size,
            (y_offset + (rows * 18 + 89) as f64) / 16.0,
            renderer,
        );
        self.dirty = true;
    }
}

impl Inventory for ChestInventory {
    fn size(&self) -> u16 {
        self.slot_count + 36
    }

    fn id(&self) -> i32 {
        self.id
    }

    fn name(&self) -> Option<&String> {
        Some(&self.name)
    }

    fn get_action_number(&self) -> i16 {
        self.action_number
    }

    fn set_action_number(&mut self, action_number: i16) {
        self.action_number = action_number;
    }

    fn get_item(&self, slot_id: u16) -> Option<Item> {
        if let Some(slot) = self.slots.get(slot_id as usize) {
            slot.item.clone()
        } else {
            let slot_id = slot_id - self.slots.len() as u16;
            self.inv_below.read().get_item(slot_id).clone()
        }
    }

    fn set_item(&mut self, slot_id: u16, item: Option<Item>) {
        let base_offset = self.slots.len() as u16;
        if slot_id < base_offset {
            self.slots[slot_id as usize].item = item;
            self.dirty = true;
        } else {
            self.inv_below.write().set_item(slot_id - base_offset, item)
        }
    }

    fn init(
        &mut self,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        inventory_window.elements.push(vec![]);
        let basic_elements = inventory_window.elements.get_mut(1).unwrap();
        let icon_scale = Hud::icon_scale(renderer.clone());
        let y = 114;
        let rows = self.slot_count / 9;
        let y_size = y + rows * 18;
        let y = (renderer.screen_data.read().safe_height as f64 / icon_scale - y_size as f64) / 2.0;
        let player_inv_img = ui::ImageBuilder::new()
            .texture_coords((0.0 / 256.0, 126.0 / 256.0, 176.0 / 256.0, 96.0 / 256.0))
            .position(0.0, icon_scale * (y + (rows * 18 + 17) as f64))
            .alignment(ui::VAttach::Top, ui::HAttach::Center)
            .size(icon_scale * 176.0, icon_scale * 96.0)
            .texture("minecraft:gui/container/generic_54")
            .create(ui_container);
        basic_elements.push(player_inv_img);
        let top_inv_img = ui::ImageBuilder::new()
            .texture_coords((
                0.0 / 256.0,
                0.0 / 256.0,
                176.0 / 256.0,
                (rows * 18 + 17) as f64 / 256.0,
            ))
            .position(0.0, icon_scale * y)
            .alignment(ui::VAttach::Top, ui::HAttach::Center)
            .size(icon_scale * 176.0, icon_scale * (rows * 18 + 17) as f64)
            .texture("minecraft:gui/container/generic_54")
            .create(ui_container);
        basic_elements.push(top_inv_img);
        let scale = icon_scale / 2.0;
        inventory_window.text_elements.push(vec![]);
        let basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();
        let title_text = ui::TextBuilder::new()
            .alignment(VAttach::Top, HAttach::Center)
            .scale_x(scale)
            .scale_y(scale)
            .position(
                icon_scale
                    * -(176.0 / 2.0
                        - 8.0
                        - renderer.ui.lock().size_of_string(self.name().unwrap()) / 4.0),
                icon_scale * (6.0 + y),
            )
            .text(self.name().unwrap())
            .colour((64, 64, 64, 255))
            .shadow(false)
            .create(ui_container);
        basic_text_elements.push(title_text);
        let inventory_text = ui::TextBuilder::new()
            .alignment(VAttach::Top, HAttach::Center)
            .scale_x(scale)
            .scale_y(scale)
            .position(
                icon_scale
                    * -(176.0 / 2.0 - 8.0 - renderer.ui.lock().size_of_string("Inventory") / 4.0),
                icon_scale * (6.0 + y + (rows * 18 + 13) as f64),
            )
            .text("Inventory")
            .colour((64, 64, 64, 255))
            .shadow(false)
            .create(ui_container);
        basic_text_elements.push(inventory_text);
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
            inventory_window.elements.get_mut(2).unwrap().clear();
            for slot in self.slots.iter() {
                if slot.item.is_some() {
                    inventory_window.draw_item_internally(
                        slot.item.as_ref().unwrap(),
                        slot.x,
                        slot.y,
                        2,
                        ui_container,
                        renderer.clone(),
                        VAttach::Top,
                    );
                }
            }
        }
    }

    fn close(&mut self) {
        // TODO
    }

    fn get_slot(&self, x: f64, y: f64) -> Option<u8> {
        for (i, slot) in self.slots.iter().enumerate() {
            if slot.contains(x, y) {
                return Some(i as u8);
            }
        }
        self.inv_below.read().get_slot(x, y).map(|i| i + self.slots.len() as u8)
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
        InventoryType::Chest((self.slot_count / 9) as u8)
    }
}
