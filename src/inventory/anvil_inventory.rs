use crate::inventory::slot_mapping::SlotMapping;
use crate::inventory::{Inventory, InventoryType, Item};
use crate::render::hud::Hud;
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use leafish_protocol::types::GameMode;
use log::warn;
use std::sync::Arc;

use leafish_protocol::protocol::packet;
use parking_lot::RwLock;

const WINDOW_WIDTH: i32 = 176;
const WINDOW_HEIGHT: i32 = 166;

pub struct AnvilInventory {
    slots: SlotMapping,
    client_state_id: i16,
    id: i32,
    last_name: String,
    repair_cost: Option<u8>,
    enough_level: bool,
    dirty: bool,
}

impl AnvilInventory {
    pub fn new(renderer: Arc<Renderer>, base_slots: Arc<RwLock<SlotMapping>>, id: i32) -> Self {
        let mut slots = SlotMapping::new((WINDOW_WIDTH, WINDOW_HEIGHT));
        slots.set_child(base_slots, (8, 84), (3..39).collect());

        slots.add_slot(0, (27, 47));
        slots.add_slot(1, (76, 47));
        slots.add_slot(2, (134, 47));

        slots.update_icons(renderer, (0, 0), None);

        Self {
            slots,
            client_state_id: 0,
            id,
            last_name: "".to_string(),
            repair_cost: None,
            enough_level: false,
            dirty: true,
        }
    }
}

impl Inventory for AnvilInventory {
    fn size(&self) -> u16 {
        self.slots.size()
    }

    fn handle_property_packet(&mut self, property: i16, value: i16) {
        if property == 0 {
            if value == 0 {
                self.repair_cost = None;
            } else {
                self.repair_cost = Some(value as u8);
            }
        } else {
            warn!("Server sent invalid data for anvil");
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
        // TODO: actually lock the slot without sending a packet
        // if we dont have enough level
        self.slots.set_item(slot_id, item);
        self.dirty = true;
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
        inventory_window.elements.push(vec![]); // Enchanting slots
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
                .texture_coords((0.0 / 256.0, 0.0 / 256.0, 176.0 / 256.0, 166.0 / 256.0))
                .position(top_left_x, top_left_y)
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(icon_scale * 176.0, icon_scale * 166.0)
                .texture("minecraft:gui/container/anvil")
                .create(ui_container),
        );

        // the name bar of anvil deactive
        basic_elements.push(
            ui::ImageBuilder::new()
                .draw_index(5)
                .texture("minecraft:gui/container/anvil")
                .texture_coords((0.0 / 256.0, 182.0 / 256.0, 110.0 / 256.0, 16.0 / 256.0))
                .position(
                    top_left_x + 59.0 * icon_scale,
                    top_left_y + 20.0 * icon_scale,
                )
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(icon_scale * 110.0, icon_scale * 16.0)
                .create(ui_container),
        );

        // anvil arrow with red cross
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture("minecraft:gui/container/anvil")
                .texture_coords((176.0 / 256.0, 0.0 / 256.0, 28.0 / 256.0, 21.0 / 256.0))
                .position(
                    top_left_x + 98.0 * icon_scale,
                    top_left_y + 44.0 * icon_scale,
                )
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(icon_scale * 28.0, icon_scale * 21.0)
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
                .text("Repair & Name")
                .colour((64, 64, 64, 255))
                .shadow(false)
                .create(ui_container),
        );

        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(icon_scale / 2.0)
                .scale_y(icon_scale / 2.0)
                .position(
                    top_left_x + 70.0 * icon_scale,
                    top_left_y + 73.0 * icon_scale,
                )
                .text("")
                .colour((104, 176, 60, 255))
                .shadow(true)
                .create(ui_container),
        );

        inventory_window.text_box.push(
            ui::TextBoxBuilder::new()
                .position(
                    top_left_x + 59.0 * icon_scale,
                    top_left_y + 20.0 * icon_scale,
                )
                .size(icon_scale * 110.0, icon_scale * 16.0)
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(icon_scale * 110.0, icon_scale * 16.0)
                .create(ui_container),
        );
        ui::TextBox::make_focusable(inventory_window.text_box.last().unwrap(), ui_container);

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
            let mut cost_text = inventory_window
                .text_elements
                .last()
                .unwrap()
                .last()
                .unwrap()
                .borrow_mut();
            // display the cost
            cost_text.text = if let Some(cost) = self.repair_cost {
                format!("Cost: {cost}")
            } else {
                format!("")
            };

            let creative_mode = inventory_window
                .inventory_context
                .read()
                .hud_context
                .read()
                .game_mode
                == GameMode::Creative;

            self.enough_level = inventory_window
                .inventory_context
                .read()
                .hud_context
                .read()
                .exp_level
                >= self.repair_cost.unwrap_or(0) as i32;

            {
                let mut arrow_crossed = inventory_window
                    .elements
                    .get_mut(0)
                    .unwrap()
                    .get_mut(2)
                    .unwrap()
                    .borrow_mut();

                if self.enough_level || creative_mode {
                    cost_text.colour = (104, 176, 60, 255);
                    if self.slots.get_item(2).is_some() {
                        arrow_crossed.colour.3 = 0;
                    } else {
                        arrow_crossed.colour.3 = 255;
                    }
                } else {
                    cost_text.colour = (255, 50, 50, 255);
                    if self.slots.get_item(2).is_some() {
                        arrow_crossed.colour.3 = 255;
                    }
                }
            }
        }

        let mut anvil_bar = inventory_window
            .elements
            .get_mut(0)
            .unwrap()
            .get_mut(1)
            .unwrap()
            .borrow_mut();

        if self.slots.get_item(0).is_some() {
            // show anvil rename bar
            anvil_bar.colour.3 = 0;
            // send name packet to server
            let current_textbox_content = inventory_window
                .text_box
                .get_mut(0)
                .unwrap()
                .borrow_mut()
                .input
                .clone();
            if current_textbox_content != self.last_name {
                self.last_name = current_textbox_content;
                inventory_window
                    .inventory_context
                    .write()
                    .get_conn()
                    .write_packet(packet::play::serverbound::NameItem {
                        item_name: self.last_name.clone(),
                    })
                    .expect("couldnt send anvil rename packet");
            }
        } else {
            // hide anvil rename bar
            anvil_bar.colour.3 = 255;
            inventory_window.text_box.last().unwrap().borrow_mut().input = "".to_string();
        }
        self.dirty = false;
    }

    fn ty(&self) -> InventoryType {
        InventoryType::Anvil
    }
}
