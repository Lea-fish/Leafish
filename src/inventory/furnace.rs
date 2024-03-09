use crate::inventory::slot_mapping::SlotMapping;
use crate::inventory::{Inventory, InventoryType, Item};
use crate::render::hud::Hud;
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use log::warn;
use std::sync::Arc;

use parking_lot::RwLock;

const WINDOW_WIDTH: i32 = 176;
const WINDOW_HEIGHT: i32 = 166;

pub struct FurnaceInventory {
    slots: SlotMapping,
    client_state_id: i16,
    ty: InventoryType,
    id: i32,
    smelting_state: SmeltingState,
}

struct SmeltingState {
    // all of these are in ticks
    remaining_burn_time: i16,
    max_burn_time: i16,
    current_progress: i16,
    max_progress: i16,
}

impl SmeltingState {
    pub fn new() -> Self {
        SmeltingState {
            remaining_burn_time: 0,
            max_burn_time: 0,
            current_progress: 0,
            max_progress: 0,
        }
    }
}

impl FurnaceInventory {
    pub fn new(
        renderer: &Arc<Renderer>,
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
            smelting_state: SmeltingState::new(),
        }
    }
}

impl Inventory for FurnaceInventory {
    fn size(&self) -> u16 {
        self.slots.size()
    }

    fn handle_property_packet(&mut self, property: i16, value: i16) {
        match property {
            0 => self.smelting_state.remaining_burn_time = value,
            1 => self.smelting_state.max_burn_time = value,
            2 => self.smelting_state.current_progress = value,
            3 => self.smelting_state.max_progress = value,
            _ => warn!("server sent invalid furnace property: {property}"),
        }
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
        renderer: &Arc<Renderer>,
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
        let icon_scale = Hud::icon_scale(renderer);

        let top_left_x =
            renderer.screen_data.read().center().0 as f64 - icon_scale * WINDOW_WIDTH as f64 / 2.0;
        let top_left_y =
            renderer.screen_data.read().center().1 as f64 - icon_scale * WINDOW_HEIGHT as f64 / 2.0;

        // Furnace texture
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((0.0, 0.0, WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64))
                .position(top_left_x, top_left_y)
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
                    top_left_y + 6.0 * icon_scale,
                )
                .text(name)
                .colour((64, 64, 64, 255))
                .shadow(false)
                .create(ui_container),
        );

        // arrow texture
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((176.0, 14.0, 0.0, 17.0))
                .texture("minecraft:gui/container/furnace")
                .size(icon_scale * 0.0, icon_scale * 17.0)
                .position(
                    top_left_x + 79.0 * icon_scale,
                    top_left_y + 35.0 * icon_scale,
                )
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .create(ui_container),
        );

        // fire texture
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((176.0, 0.0, 14.0, 0.0))
                .texture("minecraft:gui/container/furnace")
                .size(icon_scale * 14.0, icon_scale * 0.0)
                .position(
                    top_left_x + 57.0 * icon_scale,
                    top_left_y + 115.0 * icon_scale,
                )
                .alignment(ui::VAttach::Bottom, ui::HAttach::Left)
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
        let icon_scale = Hud::icon_scale(renderer);
        let basic_elements = inventory_window.elements.get_mut(0).unwrap();
        let cp = self.smelting_state.current_progress as f64;
        let mp = self.smelting_state.max_progress as f64;
        let mb = self.smelting_state.max_burn_time as f64;
        let rb = self.smelting_state.remaining_burn_time as f64;
        if cp != 0.0 {
            let mut arrow = basic_elements.get_mut(1).unwrap().borrow_mut();
            let arrow_ratio = (1.0 / (mp / cp) * 100.0).trunc() * 24.0 / 100.0;
            arrow.texture_coords = (176.0, 14.0, arrow_ratio, 17.0);
            arrow.width = arrow_ratio * icon_scale;
        }
        if rb != 0.0 {
            let mut fire = basic_elements.get_mut(2).unwrap().borrow_mut();
            let fire_ratio = (1.0 / (mb / rb) * 100.0).trunc() * 14.0 / 100.0;
            fire.texture_coords = (176.0, 14.0 - fire_ratio, 14.0, fire_ratio);
            fire.height = fire_ratio * icon_scale;
        }

        self.slots.tick(renderer, ui_container, inventory_window, 1);
    }

    fn ty(&self) -> InventoryType {
        self.ty
    }
}
