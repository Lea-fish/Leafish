use crate::inventory::slot_mapping::SlotMapping;
use crate::inventory::{Inventory, InventoryType, Item};
use crate::render::hud::Hud;
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use std::sync::Arc;

use parking_lot::RwLock;
use shared::Version;

const WINDOW_WIDTH: i32 = 176;
const WINDOW_HEIGHT: i32 = 166;

pub struct PlayerInventory {
    slots: SlotMapping,
    version: Version,
    client_state_id: i16,
}

impl PlayerInventory {
    pub fn new(
        version: Version,
        renderer: &Arc<Renderer>,
        base_slots: Arc<RwLock<SlotMapping>>,
    ) -> Self {
        let mut slots = SlotMapping::new((WINDOW_WIDTH, WINDOW_HEIGHT));
        slots.set_child(base_slots, (8, 84), (9..45).collect());

        // Crafting output
        // TODO: Use different click rules for crafting output
        slots.add_slot(0, (154, 28));

        // Crafting input
        // TODO: Reduce the count on each of these slots when output is taken
        slots.add_slot(1, (98, 18));
        slots.add_slot(2, (116, 18));
        slots.add_slot(3, (98, 36));
        slots.add_slot(4, (116, 36));

        // Armor slots
        // TODO: Only allow armor in these slots
        slots.add_slot(5, (8, 8));
        slots.add_slot(6, (8, 26));
        slots.add_slot(7, (8, 44));
        slots.add_slot(8, (8, 62));

        if version > Version::V1_8 {
            slots.add_slot(45, (77, 62));
        }

        slots.update_icons(renderer, (0, 0), None);

        Self {
            slots,
            version,
            client_state_id: 0,
        }
    }
}

impl Inventory for PlayerInventory {
    fn size(&self) -> u16 {
        self.slots.size()
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
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        inventory_window.elements.push(vec![]); // Window texture
        inventory_window.elements.push(vec![]); // Player slots
        inventory_window.elements.push(vec![]); // Base slots
        inventory_window.text_elements.push(vec![]);

        let basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();
        let basic_elements = inventory_window.elements.get_mut(0).unwrap();

        let center = renderer.screen_data.read().center();
        let icon_scale = Hud::icon_scale(renderer);

        // Inventory window
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((0.0, 0.0, WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64))
                .position(
                    center.0 as f64 - icon_scale * WINDOW_WIDTH as f64 / 2.0,
                    center.1 as f64 - icon_scale * WINDOW_HEIGHT as f64 / 2.0,
                )
                .alignment(VAttach::Top, HAttach::Left)
                .size(
                    icon_scale * WINDOW_WIDTH as f64,
                    icon_scale * WINDOW_HEIGHT as f64,
                )
                .texture("minecraft:gui/container/inventory")
                .create(ui_container),
        );

        // If before 1.9, removes the 2nd hand slot from the inv by rendering
        // the background color over it.
        if self.version < Version::V1_9 {
            basic_elements.push(
                ui::ImageBuilder::new()
                    .texture_coords(((WINDOW_WIDTH as f64 / 2.0 - 9.0), 10.0, 18.0, 18.0))
                    .position(
                        center.0 as f64 - icon_scale * (WINDOW_WIDTH as f64 / 2.0 - 76.0),
                        center.1 as f64 - icon_scale * (WINDOW_HEIGHT as f64 / 2.0 - 61.0),
                    )
                    .alignment(VAttach::Top, HAttach::Left)
                    .size(icon_scale * 18.0, icon_scale * 18.0)
                    .texture("minecraft:gui/container/inventory")
                    .create(ui_container),
            );
        }

        // Crafting text
        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(icon_scale / 2.0)
                .scale_y(icon_scale / 2.0)
                .position(
                    center.0 as f64 - icon_scale * (WINDOW_WIDTH as f64 / 2.0 - 97.0),
                    center.1 as f64 - icon_scale * (WINDOW_HEIGHT as f64 / 2.0 - 8.0),
                )
                .text("Crafting")
                .colour((64, 64, 64, 255))
                .shadow(false)
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
    }

    fn ty(&self) -> InventoryType {
        InventoryType::Main
    }
}
