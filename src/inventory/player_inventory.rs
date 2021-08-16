use crate::inventory::{Inventory, Item, InventoryType, Slot, Material};
use crate::render::Renderer;
use crate::ui::{Container, ImageRef, VAttach, HAttach};
use crate::ui;
use crate::render::hud::Hud;
use std::sync::{Arc, RwLock};
use crate::render::inventory::InventoryWindow;
use crate::server::Version;

pub struct PlayerInventory {

    slots: Vec<Slot>,
    dirty: bool,

}

impl PlayerInventory {

    const SIZE: u32 = 36; // TODO: is this actually correct?

    pub fn new(version: Version, renderer: &Renderer) -> Self {
        let scale = Hud::icon_scale(renderer) as f64;
        let size = scale / 9.0 * 16.0;
        let x_offset = -(size * 4.5);
        let y_offset = (size * 4.25);
        let hot_bar_offset = scale / 9.0 * 4.0;
        let mut slots = vec![];
        for i in 0..9 {
            slots.push(Slot::new(x_offset + i as f64 * size, y_offset, size));
        }
        for y in (0..3).rev() {
            for x in 0..9 {
                slots.push(Slot::new(x_offset + x as f64 * size, y_offset + y as f64 * size + size + hot_bar_offset, size)); // TODO: Add the little bit in between two slots
            }
        }
        PlayerInventory {
            slots,
            dirty: false
        }
    }

    fn update_icons(&mut self, renderer: &Renderer) {
        let scale = Hud::icon_scale(renderer) as f64;
        let size = scale / 9.0 * 16.0;
        let x_offset = -(size * 4.5);
        let y_offset = (size * 4.18);
        let hot_bar_offset = scale / 9.0 * 4.0;
        for i in 0..9 {
            self.slots.get_mut(i).unwrap().update_position(x_offset + i as f64 * size, y_offset, size);
        }
        for y in (0..3).rev() {
            for x in 0..9 {
                self.slots.get_mut(9 + x + 9 * (2 - y)).unwrap().update_position(x_offset + x as f64 * size, y_offset + y as f64 * size + size + hot_bar_offset, size); // TODO: Add the little bit in between two slots
            }
        }
        self.dirty = true;
    }

}

impl Inventory for PlayerInventory {

    fn size(&self) -> u32 {
        PlayerInventory::SIZE
    }

    fn id(&self) -> i32 {
        -1
    }

    fn name(&self) -> Option<&String> {
        None
    }

    fn get_item(&self, slot: u32) -> &Option<Item> {
        &self.slots[slot as usize].item
    }

    fn get_item_mut(&mut self, slot: u32) -> &mut Option<Item> {
        self.dirty = true;
        &mut self.slots[slot as usize].item
    }

    fn set_item(&mut self, slot: u32, item: Option<Item>) {
        self.slots[slot as usize].item = item;
        self.dirty = true;
    }

    fn init(&mut self, renderer: &mut Renderer, ui_container: &mut Container, inventory_window: &mut InventoryWindow) {
        inventory_window.elements.push(vec![]);
        let mut basic_elements = inventory_window.elements.get_mut(0).unwrap();
        let icon_scale = Hud::icon_scale(renderer) as f64;
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 256.0, 0.0 / 256.0, 176.0 / 256.0, 166.0 / 256.0))
            .position(0.0, 0.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .size(icon_scale / 9.0 * 176.0, icon_scale / 9.0 * 166.0)
            .texture("minecraft:gui/container/inventory")
            .create(ui_container);
        basic_elements.push(image);
        let scale = icon_scale / 9.0 / 2.0;
        inventory_window.text_elements.push(vec![]);
        let mut basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();
        let crafting_text = ui::TextBuilder::new()
            .alignment(VAttach::Middle, HAttach::Center)
            .scale_x(scale)
            .scale_y(scale)
            .position(icon_scale * 3.2, -(icon_scale * 7.80))
            .text("Crafting")
            .colour((64, 64, 64, 255))
            .shadow(false)
            .create(ui_container);
        basic_text_elements.push(crafting_text);
        inventory_window.elements.push(vec![]);
        self.slots.get_mut(0).unwrap().item.replace(Item {
            stack: Default::default(),
            material: Material::Apple
        });
        self.update_icons(renderer);
    }

    fn tick(&mut self, renderer: &mut Renderer, ui_container: &mut Container, inventory_window: &mut InventoryWindow) {
        if self.dirty {
            self.dirty = false;
            inventory_window.elements.get_mut(1).unwrap().clear();
            for slot in self.slots.iter() {
                if slot.item.is_some() {
                    inventory_window.draw_item(&slot.item.as_ref().unwrap(), slot.x, slot.y, 1, ui_container, renderer);
                }
            }
        }
    }

    fn close(&mut self, inventory_window: &mut InventoryWindow) {
        // TODO
    }

    fn click_at(&self, cursor: (u32, u32)) {
        // TODO
    }

    fn resize(&mut self, width: u32, height: u32, renderer: &mut Renderer, ui_container: &mut Container, inventory_window: &mut InventoryWindow) {
        inventory_window.clear_elements();
        self.init(renderer, ui_container, inventory_window);
    }

    fn ty(&self) -> InventoryType {
        InventoryType::Main
    }

}