use crate::inventory::base_inventory::BaseInventory;
use crate::inventory::{Inventory, InventoryType, Item, Slot};
use crate::render::hud::Hud;
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use std::sync::Arc;

use leafish_protocol::protocol::Version;
use parking_lot::RwLock;

pub struct PlayerInventory {
    slots: Vec<Slot>,
    offhand_slot: Option<Slot>,
    dirty: bool,
    version: Version,
    client_state_id: i16,
    base_inventory: Arc<RwLock<BaseInventory>>,
}

impl PlayerInventory {
    pub fn new(
        version: Version,
        renderer: Arc<Renderer>,
        base_inventory: Arc<RwLock<BaseInventory>>,
    ) -> Self {
        let scale = Hud::icon_scale(renderer);
        let size = scale * 16.0;
        let x_offset = -(size * 4.5);
        let y_offset = size * 4.25;
        let hot_bar_offset = scale * 4.0;
        let mut slots = vec![];
        let slot_craft_0 = Slot::new(
            (size + size * 1.0 / 8.0) * 2.5,
            scale * 5.0
                - (scale * 24.5 + size * 1.0 / 9.0 + size * (2.0 + 1.0 / 5.0)
                    - (size + size * 1.0 / 9.0) * 2.0),
            size,
        );
        slots.push(slot_craft_0);
        let slot_craft_1 = Slot::new(
            size + size * 1.0 / 8.0,
            scale * 5.0 - (scale * 24.5 + size * 1.0 / 9.0 + size * (2.0 + 1.0 / 5.0)),
            size,
        );
        slots.push(slot_craft_1);
        let slot_craft_2 = Slot::new(
            (size + size * 1.0 / 8.0) * 2.0,
            scale * 5.0 - (scale * 24.5 + size * 1.0 / 9.0 + size * (2.0 + 1.0 / 5.0)),
            size,
        );
        slots.push(slot_craft_2);
        let slot_craft_3 = Slot::new(
            size + size * 1.0 / 8.0,
            scale * 5.0
                - (scale * 24.5 + size * 1.0 / 9.0 + size * (2.0 + 1.0 / 5.0)
                    - (size + size * 1.0 / 9.0)),
            size,
        );
        slots.push(slot_craft_3);
        let slot_craft_4 = Slot::new(
            (size + size * 1.0 / 8.0) * 2.0,
            scale * 5.0
                - (scale * 24.5 + size * 1.0 / 9.0 + size * (2.0 + 1.0 / 5.0)
                    - (size + size * 1.0 / 9.0)),
            size,
        );
        slots.push(slot_craft_4);
        let slot_head = Slot::new(
            x_offset,
            y_offset
                + -((6_f64 + 1.0 / 8.0) * (size + size * 1.0 / 8.0) + size + hot_bar_offset * 2.0),
            size,
        ); // 6th slot!
        let slot_chestplate = Slot::new(
            x_offset,
            y_offset
                + -((5_f64 + 1.0 / 8.0) * (size + size * 1.0 / 8.0) + size + hot_bar_offset * 2.0),
            size,
        ); // 7th slot!
        let slot_leggings = Slot::new(
            x_offset,
            y_offset
                + -((4_f64 + 1.0 / 8.0) * (size + size * 1.0 / 8.0) + size + hot_bar_offset * 2.0),
            size,
        ); // 8th slot!
        let slot_boots = Slot::new(
            x_offset,
            y_offset
                + -((3_f64 + 1.0 / 8.0) * (size + size * 1.0 / 8.0) + size + hot_bar_offset * 2.0),
            size,
        ); // 9th slot!
        slots.push(slot_head);
        slots.push(slot_chestplate);
        slots.push(slot_leggings);
        slots.push(slot_boots);

        let offhand_slot = if version > Version::V1_8 {
            Some(Slot::new(-(scale * 3.0), scale * 5.0 - scale * 18.0, size))
        } else {
            None
        };
        Self {
            slots,
            offhand_slot,
            dirty: false,
            version,
            client_state_id: 0,
            base_inventory,
        }
    }

    #[allow(clippy::eq_op)]
    fn update_icons(&mut self, renderer: Arc<Renderer>) {
        let scale = Hud::icon_scale(renderer.clone());
        let base = scale * ((renderer.screen_data.read().safe_height as f64 / scale - 166.0) / 2.0);
        let middle = base + scale * 166.0 / 2.0;
        let size = scale * 16.0;
        let x_offset = -(size * 4.5);
        let slot_offset = size + size * 1.0 / 8.0;
        let slot_craft_0 = self.slots.get_mut(0).unwrap();
        slot_craft_0.update_position(
            (slot_offset) * 4.0 + size * 1.0 / 8.0,
            middle - (scale * (16.0 * 4.0 - 16.0 / 2.0 - 16.0 * 1.0 / 8.0 / 2.0)),
            size,
        );
        let slot_craft_1 = self.slots.get_mut(1).unwrap();
        slot_craft_1.update_position(
            size + size * 1.0 / 8.0,
            middle - (scale * (16.0 * 4.0 + 16.0 * 1.0 / 8.0 / 2.0)),
            size,
        );
        let slot_craft_2 = self.slots.get_mut(2).unwrap();
        slot_craft_2.update_position(
            (size + size * 1.0 / 8.0) * 2.0,
            middle - (scale * (16.0 * 4.0 + 16.0 * 1.0 / 8.0 / 2.0)),
            size,
        );
        let slot_craft_3 = self.slots.get_mut(3).unwrap();
        slot_craft_3.update_position(
            size + size * 1.0 / 8.0,
            middle - (scale * (16.0 * 4.0 + 16.0 * 1.0 / 8.0 / 2.0 - slot_offset / scale)),
            size,
        );
        let slot_craft_4 = self.slots.get_mut(4).unwrap();
        slot_craft_4.update_position(
            (size + size * 1.0 / 8.0) * 2.0,
            middle - (scale * (16.0 * 4.0 + 16.0 * 1.0 / 8.0 / 2.0 - slot_offset / scale)),
            size,
        );
        let slot_head = self.slots.get_mut(5).unwrap(); // 6th slot!
        slot_head.update_position(
            x_offset,
            middle
                - (scale
                    * (16.0 * 4.0 + 16.0 * 1.0 / 8.0 / 2.0 - slot_offset / scale * 2.5
                        + 16.0 * 1.0 / 8.0 / 2.0)
                    + slot_offset * 3.0),
            size,
        );
        let slot_chestplate = self.slots.get_mut(6).unwrap(); // 7th slot!
        slot_chestplate.update_position(
            x_offset,
            middle
                - (scale
                    * (16.0 * 4.0 + 16.0 * 1.0 / 8.0 / 2.0 - slot_offset / scale * 2.5
                        + 16.0 * 1.0 / 8.0 / 2.0)
                    + slot_offset * 2.0),
            size,
        );
        let slot_leggings = self.slots.get_mut(7).unwrap(); // 8th slot!
        slot_leggings.update_position(
            x_offset,
            middle
                - (scale
                    * (16.0 * 4.0 + 16.0 * 1.0 / 8.0 / 2.0 - slot_offset / scale * 2.5
                        + 16.0 * 1.0 / 8.0 / 2.0)
                    + slot_offset),
            size,
        );
        let slot_boots = self.slots.get_mut(8).unwrap(); // 9th slot!
        slot_boots.update_position(
            x_offset,
            middle
                - (scale
                    * (16.0 * 4.0 + 16.0 * 1.0 / 8.0 / 2.0 - slot_offset / scale * 2.5
                        + 16.0 * 1.0 / 8.0 / 2.0)),
            size,
        );
        if self.version > Version::V1_8 {
            let slot = self.offhand_slot.as_mut().unwrap();
            slot.update_position(
                -(scale * 3.0),
                middle
                    - (scale
                        * (16.0 * 4.0 + 16.0 * 1.0 / 8.0 / 2.0 - slot_offset / scale * 2.5
                            + 16.0 * 1.0 / 8.0 / 2.0)),
                size,
            );
        }
        let x_offset = -(size * 4.5);
        let screen_height = renderer.screen_data.read().safe_height as f64;
        let y_offset = (screen_height / scale) / 2.0;
        self.base_inventory.clone().write().update_offset(
            x_offset / size,
            (y_offset + 59.0) / 16.0,
            renderer,
        );
        self.dirty = true;
    }
}

impl Inventory for PlayerInventory {
    fn size(&self) -> u16 {
        let mut size = self.slots.len() as u16;
        size += self.base_inventory.read().size();
        if self.offhand_slot.is_some() {
            size += 1;
        }
        size
    }

    fn id(&self) -> i32 {
        // The player inventory always uses the id 0.
        // See: https://wiki.vg/Protocol#Set_Container_Content
        0
    }

    fn get_client_state_id(&self) -> i16 {
        self.client_state_id
    }

    fn set_client_state_id(&mut self, client_state_id: i16) {
        self.client_state_id = client_state_id;
    }

    fn get_item(&self, slot_id: u16) -> Option<Item> {
        if let Some(slot) = self.slots.get(slot_id as usize) {
            slot.item.clone()
        } else if slot_id == 45 {
            self.offhand_slot.as_ref().unwrap().item.clone()
        } else {
            let slot_id = slot_id - self.slots.len() as u16;
            self.base_inventory.read().get_item(slot_id)
        }
    }

    fn set_item(&mut self, slot_id: u16, item: Option<Item>) {
        if let Some(slot) = self.slots.get_mut(slot_id as usize) {
            slot.item = item;
            self.dirty = true;
        } else if slot_id == 45 {
            self.offhand_slot.as_mut().unwrap().item = item;
            self.dirty = true;
        } else {
            let slot_id = slot_id - self.slots.len() as u16;
            self.base_inventory.write().set_item(slot_id, item);
        }
    }

    #[allow(clippy::eq_op)]
    fn init(
        &mut self,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        inventory_window.elements.push(vec![]);
        let basic_elements = inventory_window.elements.get_mut(1).unwrap();
        let icon_scale = Hud::icon_scale(renderer.clone());
        let size = icon_scale * 16.0;
        let slot_offset = size + size * 1.0 / 8.0;
        let base = icon_scale
            * ((renderer.screen_data.read().safe_height as f64 / icon_scale - 166.0) / 2.0);
        let middle = base + icon_scale * 166.0 / 2.0;
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 256.0, 0.0 / 256.0, 176.0 / 256.0, 166.0 / 256.0))
            .position(0.0, base)
            .alignment(ui::VAttach::Top, ui::HAttach::Center)
            .size(icon_scale * 176.0, icon_scale * 166.0)
            .texture("minecraft:gui/container/inventory")
            .create(ui_container);
        basic_elements.push(image);
        if self.version < Version::V1_9 {
            // Removes the 2nd hand slot from the inv by rendering the background color over it.
            let image = ui::ImageBuilder::new()
                .texture_coords((
                    (176.0 / 2.0 - 9.0) / 256.0,
                    10.0 / 256.0,
                    18.0 / 256.0,
                    18.0 / 256.0,
                ))
                .position(
                    -(icon_scale * 3.0),
                    middle
                        - (icon_scale
                            * (16.0 * 4.0 + 16.0 * 1.0 / 8.0 / 2.0
                                - slot_offset / icon_scale * 2.5
                                + 16.0 * 1.0 / 8.0)),
                )
                .alignment(ui::VAttach::Top, ui::HAttach::Center)
                .size(icon_scale * 18.0, icon_scale * 18.0)
                .texture("minecraft:gui/container/inventory")
                .create(ui_container);
            basic_elements.push(image);
        }
        inventory_window.text_elements.push(vec![]);
        let basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();
        let crafting_text = ui::TextBuilder::new()
            .alignment(VAttach::Top, HAttach::Center)
            .scale_x(icon_scale / 2.0)
            .scale_y(icon_scale / 2.0)
            .position(
                icon_scale * 9.0 * 3.2,
                middle - (icon_scale * (16.0 * 4.0 + 11.0)),
            )
            .text("Crafting")
            .colour((64, 64, 64, 255))
            .shadow(false)
            .create(ui_container);
        basic_text_elements.push(crafting_text);
        inventory_window.elements.push(vec![]);
        self.update_icons(renderer);
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
            for slot in &self.slots {
                if let Some(item) = &slot.item {
                    inventory_window.draw_item_internally(
                        item,
                        slot.x,
                        slot.y,
                        2,
                        ui_container,
                        renderer.clone(),
                        VAttach::Top,
                    );
                }
            }
            if let Some(slot) = &self.offhand_slot {
                if let Some(item) = &slot.item {
                    inventory_window.draw_item_internally(
                        item,
                        slot.x,
                        slot.y,
                        2,
                        ui_container,
                        renderer,
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

        if let Some(slot) = &self.offhand_slot {
            if slot.is_within(x, y) {
                return Some(45);
            }
        }

        self.base_inventory
            .read()
            .get_slot(x, y)
            .map(|i| i + self.slots.len() as u8)
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
        InventoryType::Main
    }
}
