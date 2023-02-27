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

pub struct FurnaceInventory {
    slots: SlotMapping,
    client_state_id: i16,
    ty: InventoryType,
    id: i32,
}

impl FurnaceInventory {
    pub fn new(
        renderer: Arc<Renderer>,
        base_slots: Arc<RwLock<SlotMapping>>,
        ty: InventoryType,
        id: i32,
    ) -> Self {
        let mut slots = SlotMapping::new((WINDOW_WIDTH, WINDOW_HEIGHT));
        slots.set_child(base_slots, (8, 84), (3..39).collect());

        // Ingredient slot
        slots.add_slot(0, (56, 17));

        // Fuel slot
        slots.add_slot(1, (56, 53));

        // Output slot
        slots.add_slot(2, (116, 35));

        slots.update_icons(renderer, (0, 0), None);

        Self {
            slots,
            client_state_id: 0,
            ty,
            id,
        }
    }
}

impl Inventory for FurnaceInventory {
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
        inventory_window.elements.push(vec![]); // Furnace slots
        inventory_window.elements.push(vec![]); // Base slots
        inventory_window.text_elements.push(vec![]);

        let basic_elements = inventory_window.elements.get_mut(0).unwrap();
        let basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();

        let center = renderer.screen_data.read().center();
        let icon_scale = Hud::icon_scale(renderer.clone());

        // Furnace texture
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((
                    0.0 / 256.0,
                    0.0 / 256.0,
                    WINDOW_WIDTH as f64 / 256.0,
                    WINDOW_HEIGHT as f64 / 256.0,
                ))
                .position(
                    center.0 as f64 - icon_scale * WINDOW_WIDTH as f64 / 2.0,
                    center.1 as f64 - icon_scale * WINDOW_HEIGHT as f64 / 2.0,
                )
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(
                    icon_scale * WINDOW_WIDTH as f64,
                    icon_scale * WINDOW_HEIGHT as f64,
                )
                .texture("minecraft:gui/container/furnace")
                .create(ui_container),
        );

        // Title text
        let name = match self.ty {
            InventoryType::Furnace => "Furnace",
            InventoryType::BlastFurnace => "Blast Furnace",
            InventoryType::Smoker => "Smoker",
            _ => unreachable!(),
        };
        let title_offset = renderer.ui.lock().size_of_string(name) / 4.0;
        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(icon_scale / 2.0)
                .scale_y(icon_scale / 2.0)
                .position(
                    center.0 as f64 - icon_scale * title_offset.ceil(),
                    center.1 as f64 - icon_scale * (WINDOW_HEIGHT as f64 / 2.0 - 6.0),
                )
                .text(name)
                .colour((64, 64, 64, 255))
                .shadow(false)
                .create(ui_container),
        );

        // Inventory text
        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(icon_scale / 2.0)
                .scale_y(icon_scale / 2.0)
                .position(
                    center.0 as f64 - icon_scale * (WINDOW_WIDTH as f64 / 2.0 - 8.0),
                    center.1 as f64 - icon_scale * (WINDOW_HEIGHT as f64 / 2.0 - 72.0),
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
        // TODO: Render fuel level and smelting progress
        self.slots.tick(renderer, ui_container, inventory_window, 1);
    }

    fn ty(&self) -> InventoryType {
        self.ty
    }
}
