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

pub struct BrewingStandInventory {
    slots: SlotMapping,
    name: String,
    id: i32,
    client_state_id: i16,
    brew_time: u16,
    last_brew_time: u16,
    fuel_time: u8,
    dirty: bool,
}

impl BrewingStandInventory {
    pub fn new(
        renderer: Arc<Renderer>,
        base_slots: Arc<RwLock<SlotMapping>>,
        name: String,
        id: i32,
    ) -> Self {
        let mut slots = SlotMapping::new((WINDOW_WIDTH, WINDOW_HEIGHT));
        slots.set_child(base_slots, (8, 84), (5..41).collect());

        slots.add_slot(0, (56, 51));
        slots.add_slot(1, (79, 58));
        slots.add_slot(2, (102, 51));
        slots.add_slot(3, (79, 17));
        slots.add_slot(4, (17, 17));

        slots.update_icons(renderer, (0, 0), None);

        Self {
            slots,
            client_state_id: 0,
            name,
            id,
            brew_time: 400,
            last_brew_time: 0,
            fuel_time: 0,
            dirty: true,
        }
    }
}

impl Inventory for BrewingStandInventory {
    fn size(&self) -> u16 {
        self.slots.size()
    }

    fn handle_property_packet(&mut self, property: i16, value: i16) {
        match property {
            0 => {
                self.last_brew_time = self.brew_time;
                self.brew_time = value as u16;
            }
            1 => self.fuel_time = value as u8,
            _ => warn!("server sent invalid data for brewing stand"),
        }
        self.dirty = true;
    }

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
        self.dirty = true;
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

        let icon_scale = Hud::icon_scale(renderer.clone());
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
                .texture("minecraft:gui/container/brewing_stand")
                .create(ui_container),
        );

        // fuel bar
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture("minecraft:gui/container/brewing_stand")
                .texture_coords((176.0, 29.0, 18.0, 4.0))
                .position(
                    top_left_x + 60.0 * icon_scale,
                    top_left_y + 44.0 * icon_scale,
                )
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(icon_scale * 18.0, icon_scale * 4.0)
                .create(ui_container),
        );

        // bubbles
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture("minecraft:gui/container/brewing_stand")
                .texture_coords((186.0, 0.0, 11.0, 29.0))
                .position(
                    top_left_x + 64.0 * icon_scale,
                    top_left_y + 13.0 * icon_scale,
                )
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(icon_scale * 11.0, icon_scale * 29.0)
                .create(ui_container),
        );

        // arrow
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture("minecraft:gui/container/brewing_stand")
                .texture_coords((177.0, 0.0, 9.0, 28.0))
                .position(
                    top_left_x + 98.0 * icon_scale,
                    top_left_y + 16.0 * icon_scale,
                )
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(icon_scale * 9.0, icon_scale * 28.0)
                .create(ui_container),
        );

        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(icon_scale / 2.0)
                .scale_y(icon_scale / 2.0)
                .position(
                    top_left_x + 60.0 * icon_scale,
                    top_left_y + 6.0 * icon_scale,
                )
                .text(self.name.to_string())
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
        self.slots
            .tick(renderer.clone(), ui_container, inventory_window, 1);
        if self.dirty {
            self.dirty = false;
            let icon_scale = Hud::icon_scale(renderer);

            // fuel meter
            {
                let mut fuel_bar = inventory_window
                    .elements
                    .first_mut()
                    .unwrap()
                    .get_mut(1)
                    .unwrap()
                    .borrow_mut();
                fuel_bar.width = self.fuel_time as f64 * 0.9 * icon_scale;
                fuel_bar.texture_coords.3 = fuel_bar.width / icon_scale;
            }

            {
                let mut arrow = inventory_window
                    .elements
                    .first_mut()
                    .unwrap()
                    .get_mut(3)
                    .unwrap()
                    .borrow_mut();
                arrow.height = (400 - self.brew_time) as f64 * icon_scale * (28.0 / 400.0);
                arrow.texture_coords.3 = arrow.height / icon_scale;
            }

            // bubble animation
            if self.last_brew_time != self.brew_time {
                let mut bubbles = inventory_window
                    .elements
                    .first_mut()
                    .unwrap()
                    .get_mut(2)
                    .unwrap()
                    .borrow_mut();
                bubbles.height += 1.0 * icon_scale;
                bubbles.texture_coords.3 += 1.0;
                if bubbles.texture_coords.3 >= 29.0 {
                    bubbles.height = 0.0;
                    bubbles.texture_coords.3 = 0.0;
                }
            }
        }
    }

    fn ty(&self) -> InventoryType {
        InventoryType::BrewingStand
    }
}
