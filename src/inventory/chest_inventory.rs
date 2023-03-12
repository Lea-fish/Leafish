use crate::inventory::slot_mapping::SlotMapping;
use crate::inventory::{Inventory, InventoryType, Item};
use crate::render::hud::Hud;
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use std::sync::Arc;

use parking_lot::RwLock;

const WINDOW_WIDTH: i32 = 176;
const SLOT_SIZE: i32 = 18;
const INVENTORY_HEIGHT: i32 = 96;

fn chest_height(rows: u8) -> i32 {
    rows as i32 * SLOT_SIZE + 17
}

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
        let mut slots = SlotMapping::new((WINDOW_WIDTH, chest_height(rows) + INVENTORY_HEIGHT));
        let child_range = ((rows as u16 * 9)..(rows as u16 * 9 + 36)).collect();
        slots.set_child(base_slots, (8, chest_height(rows) + 14), child_range);

        for y in 0..rows as i32 {
            for x in 0..9 {
                slots.add_slot((x + y * 9) as u16, (8 + x * SLOT_SIZE, (y + 1) * SLOT_SIZE));
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
        let basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();

        let icon_scale = Hud::icon_scale(renderer.clone()) as i32;
        let chest_height_scaled = icon_scale * chest_height(self.rows);
        let inventory_height_scaled = icon_scale * INVENTORY_HEIGHT;
        let total_height_scaled = chest_height_scaled + inventory_height_scaled;
        let total_width_scaled = icon_scale * WINDOW_WIDTH;
        let center = renderer.screen_data.read().center();

        // Chest section
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((
                    0.0,
                    0.0,
                    WINDOW_WIDTH as f64,
                    chest_height(self.rows) as f64,
                ))
                .position(
                    (center.0 as i32 - total_width_scaled / 2) as f64,
                    (center.1 as i32 - total_height_scaled / 2) as f64,
                )
                .alignment(VAttach::Top, HAttach::Left)
                .size(total_width_scaled as f64, chest_height_scaled as f64)
                .texture("minecraft:gui/container/generic_54")
                .create(ui_container),
        );

        // Player inventory
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((
                    0.0,
                    (chest_height(6) + 1) as f64,
                    WINDOW_WIDTH as f64,
                    INVENTORY_HEIGHT as f64,
                ))
                .position(
                    (center.0 as i32 - total_width_scaled / 2) as f64,
                    (center.1 as i32 - total_height_scaled / 2 + chest_height_scaled) as f64,
                )
                .alignment(VAttach::Top, HAttach::Left)
                .size(total_width_scaled as f64, inventory_height_scaled as f64)
                .texture("minecraft:gui/container/generic_54")
                .create(ui_container),
        );

        // Title text
        let title = match self.name.as_str() {
            "container.barrel" => "Barrel",
            "container.chest" => "Chest",
            "container.chestDouble" => "Large Chest",
            "container.enderchest" => "Ender Chest",
            name => name,
        };
        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(icon_scale as f64 / 2.0)
                .scale_y(icon_scale as f64 / 2.0)
                .position(
                    (center.0 as i32 - total_width_scaled / 2 + icon_scale * 8) as f64,
                    (center.1 as i32 - total_height_scaled / 2 + icon_scale * 6) as f64,
                )
                .text(title)
                .colour((64, 64, 64, 255))
                .shadow(false)
                .create(ui_container),
        );

        // Inventory text
        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(icon_scale as f64 / 2.0)
                .scale_y(icon_scale as f64 / 2.0)
                .position(
                    (center.0 as i32 - total_width_scaled / 2 + icon_scale * 8) as f64,
                    (center.1 as i32 - total_height_scaled / 2
                        + icon_scale * 3
                        + chest_height_scaled) as f64,
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

    fn ty(&self) -> InventoryType {
        InventoryType::Chest(self.rows)
    }
}
