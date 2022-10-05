use crate::inventory::slot_mapping::SlotMapping;
use crate::inventory::{Inventory, Item};
use crate::render::hud::Hud;
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use std::sync::Arc;

use leafish_protocol::protocol::Version;
use parking_lot::RwLock;

pub struct PlayerInventory {
    slots: SlotMapping,
    version: Version,
    client_state_id: i16,
}

impl PlayerInventory {
    pub fn new(
        version: Version,
        renderer: Arc<Renderer>,
        base_slots: Arc<RwLock<SlotMapping>>,
    ) -> Self {
        let mut slots = SlotMapping::new((176, 166));
        slots.set_child(base_slots, (8, 84), (9..45).collect());

        // Crafting output
        slots.add_slot(0, (154, 28));

        // Crafting input
        slots.add_slot(1, (98, 18));
        slots.add_slot(2, (116, 18));
        slots.add_slot(3, (98, 36));
        slots.add_slot(4, (116, 36));

        // Armor slots
        // TODO: Only allow armor in these slots
        slots.add_slot(5, (8, 8));
        slots.add_slot(6, (8, 26));
        slots.add_slot(7, (8, 44));
        slots.add_slot(8, (8, 80));

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

    #[allow(clippy::eq_op)]
    fn init(
        &mut self,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        inventory_window.elements.push(vec![]);
        let center = {
            let size = renderer.screen_data.read();
            (size.safe_width as i32 / 2, size.safe_height as i32 / 2)
        };
        let basic_elements = inventory_window.elements.get_mut(0).unwrap();
        let icon_scale = Hud::icon_scale(renderer.clone());
        let size = icon_scale * 16.0;
        let slot_offset = size + size * 1.0 / 8.0;
        let base = icon_scale
            * ((renderer.screen_data.read().safe_height as f64 / icon_scale - 166.0) / 2.0);
        let middle = base + icon_scale * 166.0 / 2.0;
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 256.0, 0.0 / 256.0, 176.0 / 256.0, 166.0 / 256.0))
            .position(
                center.0 as f64 - icon_scale * 176.0 / 2.0,
                center.1 as f64 - icon_scale * 166.0 / 2.0,
            )
            .alignment(ui::VAttach::Top, ui::HAttach::Left)
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
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
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
        inventory_window.elements.push(vec![]); // For player slots
        inventory_window.elements.push(vec![]); // For base slots
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
