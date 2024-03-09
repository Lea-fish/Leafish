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
const WINDOW_HEIGHT: i32 = 166;

pub struct GrindStoneInventory {
    slots: SlotMapping,
    client_state_id: i16,
    id: i32,
    dirty: bool,
}

impl GrindStoneInventory {
    pub fn new(renderer: &Arc<Renderer>, base_slots: Arc<RwLock<SlotMapping>>, id: i32) -> Self {
        let mut slots = SlotMapping::new((WINDOW_WIDTH, WINDOW_HEIGHT));
        slots.set_child(base_slots, (8, 84), (3..39).collect());

        slots.add_slot(0, (49, 19));
        slots.add_slot(1, (49, 40));
        slots.add_slot(2, (129, 34));

        slots.update_icons(renderer, (0, 0), None);

        Self {
            slots,
            client_state_id: 0,
            id,
            dirty: true,
        }
    }
}

impl Inventory for GrindStoneInventory {
    fn size(&self) -> u16 {
        self.slots.size()
    }

    fn handle_property_packet(&mut self, _property: i16, _value: i16) {}

    fn id(&self) -> i32 {
        self.id
    }

    fn get_client_state_id(&self) -> i16 {
        self.client_state_id
    }

    fn set_client_state_id(&mut self, client_state_id: i16) {
        self.dirty = true;
        self.client_state_id = client_state_id;
    }

    fn get_item(&self, slot_id: u16) -> Option<Item> {
        self.slots.get_item(slot_id)
    }

    fn set_item(&mut self, slot_id: u16, item: Option<Item>) {
        self.slots.set_item(slot_id, item);
        self.dirty = true;
    }

    fn get_slot(&self, x: f64, y: f64) -> Option<u16> {
        self.slots.get_slot(x, y)
    }

    fn init(
        &mut self,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        inventory_window.elements.push(vec![]); // Window texture
        inventory_window.elements.push(vec![]); // Enchanting slots
        inventory_window.elements.push(vec![]); // Base slots
        inventory_window.text_elements.push(vec![]);

        let basic_elements = inventory_window.elements.get_mut(0).unwrap();
        let basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();
        let icon_scale = Hud::icon_scale(renderer);

        let top_left_x =
            renderer.screen_data.read().center().0 as f64 - icon_scale * WINDOW_WIDTH as f64 / 2.0;
        let top_left_y =
            renderer.screen_data.read().center().1 as f64 - icon_scale * WINDOW_HEIGHT as f64 / 2.0;
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((0.0, 0.0, 176.0, 166.0))
                .position(top_left_x, top_left_y)
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(icon_scale * 176.0, icon_scale * 166.0)
                .texture("minecraft:gui/container/grindstone")
                .create(ui_container),
        );

        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(icon_scale / 2.0)
                .scale_y(icon_scale / 2.0)
                .position(
                    top_left_x + 11.0 * icon_scale,
                    top_left_y + 5.0 * icon_scale,
                )
                .text("Repair & Disenchant")
                .colour((64, 64, 64, 255))
                .shadow(false)
                .create(ui_container),
        );

        basic_elements.push(
            ui::ImageBuilder::new()
                .texture("minecraft:gui/container/grindstone")
                .texture_coords((176.0, 0.0, 28.0, 21.0))
                .position(
                    top_left_x + 92.0 * icon_scale,
                    top_left_y + 31.0 * icon_scale,
                )
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(icon_scale * 28.0, icon_scale * 21.0)
                .create(ui_container),
        );

        self.slots.update_icons(renderer, (0, 0), None);
    }

    fn tick(
        &mut self,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        self.slots.tick(renderer, ui_container, inventory_window, 1);
        if self.dirty {
            let mut arrow_crossed = inventory_window
                .elements
                .get_mut(0)
                .unwrap()
                .get_mut(1)
                .unwrap()
                .borrow_mut();
            if self.slots.get_item(2).is_none()
                && (self.slots.get_item(0).is_some() || self.slots.get_item(1).is_some())
            {
                arrow_crossed.colour.3 = 255;
            } else {
                arrow_crossed.colour.3 = 0;
            }
        }
    }

    fn ty(&self) -> InventoryType {
        InventoryType::Grindstone
    }
}
