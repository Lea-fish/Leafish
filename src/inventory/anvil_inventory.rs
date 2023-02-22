use crate::inventory::slot_mapping::SlotMapping;
use crate::inventory::{Inventory, InventoryType, Item};
use crate::render::hud::Hud;
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
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
        inventory_window.elements.push(vec![]); // Enchanting slots
        inventory_window.elements.push(vec![]); // Base slots
        inventory_window.text_elements.push(vec![]);

        let basic_elements = inventory_window.elements.get_mut(0).unwrap();
        let basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();

        let center = renderer.screen_data.read().center();
        let icon_scale = Hud::icon_scale(renderer.clone());

        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((0.0 / 256.0, 0.0 / 256.0, 176.0 / 256.0, 166.0 / 256.0))
                .position(
                    center.0 as f64 - icon_scale * WINDOW_WIDTH as f64 / 2.0,
                    center.1 as f64 - icon_scale * WINDOW_HEIGHT as f64 / 2.0,
                )
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
                    center.0 as f64 - icon_scale * WINDOW_WIDTH as f64 / 2.0 + 59.0 * icon_scale,
                    center.1 as f64 - icon_scale * WINDOW_HEIGHT as f64 / 2.0 + 20.0 * icon_scale,
                )
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(icon_scale * 110.0, icon_scale * 16.0)
                .create(ui_container),
        );

        // the name bar of anvil active
        // basic_elements.push(
        //     ui::ImageBuilder::new()
        //         .draw_index(0)
        //         .texture("minecraft:gui/container/anvil")
        //         .texture_coords((0.0 / 256.0, 166.0 / 256.0, 110.0 / 256.0, 16.0 / 256.0))
        //         .position(
        //             center.0 as f64 - icon_scale * WINDOW_WIDTH as f64 / 2.0 + 59.0 * icon_scale,
        //             center.1 as f64 - icon_scale * WINDOW_HEIGHT as f64 / 2.0 + 20.0 * icon_scale,
        //         )
        //         .alignment(ui::VAttach::Top, ui::HAttach::Left)
        //         .size(icon_scale * 110.0, icon_scale * 16.0)
        //         .create(ui_container),
        // );

        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(icon_scale / 2.0)
                .scale_y(icon_scale / 2.0)
                .position(
                    center.0 as f64 - icon_scale * (WINDOW_WIDTH as f64 / 2.0 - 68.0),
                    center.1 as f64 - icon_scale * (WINDOW_HEIGHT as f64 / 2.0 - 6.0),
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
                    center.0 as f64 - icon_scale * WINDOW_WIDTH as f64 / 2.0 + 70.0 * icon_scale,
                    center.1 as f64 - icon_scale * WINDOW_HEIGHT as f64 / 2.0 + 73.0 * icon_scale,
                )
                .text("")
                .colour((104, 176, 60, 255))
                .shadow(true)
                .create(ui_container),
        );

        inventory_window.text_box.push(
            ui::TextBoxBuilder::new()
                .position(
                    center.0 as f64 - icon_scale * WINDOW_WIDTH as f64 / 2.0 + 59.0 * icon_scale,
                    center.1 as f64 - icon_scale * WINDOW_HEIGHT as f64 / 2.0 + 20.0 * icon_scale,
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

        if let Some(cost) = self.repair_cost {
            inventory_window
                .text_elements
                .last()
                .unwrap()
                .last()
                .unwrap()
                .borrow_mut()
                .text = format!("Cost: {cost}");
        } else {
            inventory_window
                .text_elements
                .last()
                .unwrap()
                .last()
                .unwrap()
                .borrow_mut()
                .text = format!("");
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
                    .conn
                    .write()
                    .clone()
                    .unwrap()
                    .write_packet(packet::play::serverbound::NameItem {
                        item_name: inventory_window
                            .text_box
                            .last()
                            .unwrap()
                            .borrow_mut()
                            .input
                            .clone(),
                    })
                    .unwrap();
            }
        } else {
            // hide anvil rename bar
            anvil_bar.colour.3 = 255;
            inventory_window.text_box.last().unwrap().borrow_mut().input = "".to_string();
        }
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

    fn ty(&self) -> InventoryType {
        InventoryType::Anvil
    }
}
