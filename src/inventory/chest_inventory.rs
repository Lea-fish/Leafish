use crate::inventory::slot_mapping::SlotMapping;
use crate::inventory::{Inventory, Item};
use crate::render::hud::Hud;
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use std::sync::Arc;

use parking_lot::RwLock;

pub struct ChestInventory {
    slots: SlotMapping,
    name: String,
    rows: u8,
    id: i32,
    client_state_id: i16,
}

impl ChestInventory {
    pub fn new(
        renderer: Arc<Renderer>,
        base_slots: Arc<RwLock<SlotMapping>>,
        rows: u8,
        name: String,
        id: i32,
    ) -> Self {
        let mut slots = SlotMapping::new((176, 113 + 18 * rows as i32));
        let child_range = ((rows as u16 * 9)..(rows as u16 * 9 + 36)).collect();
        slots.set_child(base_slots, (8, 31 + 18 * rows as i32), child_range);

        for y in 0..rows as i32 {
            for x in 0..9 {
                slots.add_slot((x + y * 9) as u16, (8 + x * 18, (y + 1) * 18));
            }
        }

        slots.update_icons(renderer, (0, 0), None);

        Self {
            slots,
            rows,
            name,
            id,
            client_state_id: 0,
        }
    }
}

impl Inventory for ChestInventory {
    fn size(&self) -> u16 {
        self.slots.size()
    }

    fn id(&self) -> i32 {
        self.id
    }

    fn get_client_state_id(&self) -> i16 {
        self.client_state_id
    }

    fn set_client_state_id(&mut self, client_state_id: i16) {
        self.client_state_id = client_state_id;
    }

    fn get_item(&self, slot_id: u16) -> Option<Item> {
        self.slots.get_item(slot_id)
    }

    fn set_item(&mut self, slot_id: u16, item: Option<Item>) {
        self.slots.set_item(slot_id, item);
    }

    fn get_slot(&self, x: f64, y: f64) -> Option<u16> {
        self.slots.get_slot(x, y)
    }

    fn init(
        &mut self,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        inventory_window.elements.push(vec![]); // Window texture
        inventory_window.elements.push(vec![]); // Chest slots
        inventory_window.elements.push(vec![]); // Base slots
        inventory_window.text_elements.push(vec![]);

        let basic_elements = inventory_window.elements.get_mut(0).unwrap();
        let icon_scale = Hud::icon_scale(renderer.clone()) as i32;
        let chest_grid_height = icon_scale * (self.rows as i32 * 18 + 17);
        let inventory_grid_height = icon_scale * 96;
        let total_height = chest_grid_height + inventory_grid_height;
        let total_width = icon_scale * 176;
        let center = renderer.screen_data.read().center();

        // Chest section
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((
                    0.0 / 256.0,
                    0.0 / 256.0,
                    176.0 / 256.0,
                    (self.rows * 18 + 17) as f64 / 256.0,
                ))
                .position(
                    (center.0 as i32 - total_width / 2) as f64,
                    (center.1 as i32 - total_height / 2) as f64,
                )
                .alignment(VAttach::Top, HAttach::Left)
                .size(total_width as f64, chest_grid_height as f64)
                .texture("minecraft:gui/container/generic_54")
                .create(ui_container),
        );
        // Player inventory
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((0.0 / 256.0, 126.0 / 256.0, 176.0 / 256.0, 96.0 / 256.0))
                .position(
                    (center.0 as i32 - total_width / 2) as f64,
                    (center.1 as i32 - total_height / 2 + chest_grid_height) as f64,
                )
                .alignment(VAttach::Top, HAttach::Left)
                .size(total_width as f64, inventory_grid_height as f64)
                .texture("minecraft:gui/container/generic_54")
                .create(ui_container),
        );

        let scale = icon_scale as f64 / 2.0;
        let basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();
        // Title text
        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(scale)
                .scale_y(scale)
                .position(
                    (center.0 as i32 - total_width / 2 + icon_scale * 8) as f64,
                    (center.1 as i32 - total_height / 2 + icon_scale * 6) as f64,
                )
                .text(&self.name)
                .colour((64, 64, 64, 255))
                .shadow(false)
                .create(ui_container),
        );
        // Inventory text
        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(scale)
                .scale_y(scale)
                .position(
                    (center.0 as i32 - total_width / 2 + icon_scale * 8) as f64,
                    (center.1 as i32 - total_height / 2 + icon_scale * 3 + chest_grid_height)
                        as f64,
                )
                .text("Inventory")
                .colour((64, 64, 64, 255))
                .shadow(false)
                .create(ui_container),
        );

        self.slots.update_icons(renderer, (0, 0), None);
    }

    fn tick(
        &mut self,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        self.slots.tick(renderer, ui_container, inventory_window, 1);
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
}
