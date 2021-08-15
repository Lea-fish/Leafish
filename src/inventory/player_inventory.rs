use crate::inventory::{Inventory, Item, InventoryType};
use crate::render::Renderer;
use crate::ui::{Container, ImageRef};
use crate::ui;
use crate::render::hud::Hud;
use std::sync::{Arc, RwLock};
use crate::render::inventory::InventoryWindow;

pub struct PlayerInventory {

    items: [Option<Item>; PlayerInventory::SIZE as usize],

}

impl PlayerInventory {

    const SIZE: u32 = 36; // TODO: is this actually correct?

    pub fn new() -> Self {
        PlayerInventory {
            items: [None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,],
        }
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
        &self.items[slot as usize]
    }

    fn get_item_mut(&mut self, slot: u32) -> &mut Option<Item> {
        &mut self.items[slot as usize]
    }

    fn set_item(&mut self, slot: u32, item: Option<Item>) {
        self.items[slot as usize] = item;
    }

    fn init(&mut self, renderer: &mut Renderer, ui_container: &mut Container, inventory_window: &mut InventoryWindow) {
        inventory_window.elements.push(vec![]);
        let mut basic_elements = inventory_window.elements.get_mut(0).unwrap();
        let icon_scale = Hud::icon_scale(renderer) as f64;
        let image = ui::ImageBuilder::new()
            .texture_coords((176.0 / 256.0, 166.0 / 256.0, 182.0 / 256.0, 22.0 / 256.0))
            .position(0.0, 0.0)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Center)
            .size(icon_scale / 9.0 * 182.0, icon_scale / 9.0 * 22.0)
            .texture("minecraft:gui/container/inventory")
            .create(ui_container);
        basic_elements.push(image);
    }

    fn tick(&mut self, renderer: &mut Renderer, ui_container: &mut Container, inventory_window: &mut InventoryWindow) {
        // TODO
    }

    fn close(&mut self) {
        // TODO
    }

    fn click_at(&self, cursor: (u32, u32)) {
        // TODO
    }

    fn ty(&self) -> InventoryType {
        InventoryType::Main
    }

}