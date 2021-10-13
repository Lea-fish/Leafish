use crate::inventory::{Inventory, InventoryType, Item, Material, Slot};
use crate::render::hud::{Hud, HudContext};
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use std::sync::Arc;

use leafish_protocol::protocol::Version;
use parking_lot::RwLock;
use std::sync::atomic::Ordering;

pub struct PlayerInventory {
    slots: Vec<Slot>,
    dirty: bool,
    version: Version,
    hud_context: Arc<RwLock<HudContext>>,
}

impl PlayerInventory {
    pub fn new(
        version: Version,
        renderer: &Renderer,
        hud_context: Arc<RwLock<HudContext>>,
    ) -> Self {
        let scale = Hud::icon_scale(renderer);
        let size = scale * 16.0;
        let x_offset = -(size * 4.5);
        let y_offset = size * 4.25;
        let hot_bar_offset = scale * 4.0;
        let mut slots = vec![];
        let mut slot_craft_0 = Slot::new(
            (size + size * 1.0 / 8.0) * 2.5,
            scale * 5.0
                - (scale * 24.5 + size * 1.0 / 9.0 + size * (2.0 + 1.0 / 5.0)
                    - (size + size * 1.0 / 9.0) * 2.0),
            size,
        );
        slot_craft_0.item = Some(Item {
            // TODO: The 0th slot isn't rendered, no matter what! FIX THIS!
            stack: Default::default(),
            material: Material::Apple,
        });
        slots.push(slot_craft_0);
        let mut slot_craft_1 = Slot::new(
            size + size * 1.0 / 8.0,
            scale * 5.0 - (scale * 24.5 + size * 1.0 / 9.0 + size * (2.0 + 1.0 / 5.0)),
            size,
        );
        slot_craft_1.item = Some(Item {
            stack: Default::default(),
            material: Material::Apple,
        });
        slots.push(slot_craft_1);
        let mut slot_craft_2 = Slot::new(
            (size + size * 1.0 / 8.0) * 2.0,
            scale * 5.0 - (scale * 24.5 + size * 1.0 / 9.0 + size * (2.0 + 1.0 / 5.0)),
            size,
        );
        slot_craft_2.item = Some(Item {
            stack: Default::default(),
            material: Material::Apple,
        });
        slots.push(slot_craft_2);
        let mut slot_craft_3 = Slot::new(
            size + size * 1.0 / 8.0,
            scale * 5.0
                - (scale * 24.5 + size * 1.0 / 9.0 + size * (2.0 + 1.0 / 5.0)
                    - (size + size * 1.0 / 9.0)),
            size,
        );
        slot_craft_3.item = Some(Item {
            stack: Default::default(),
            material: Material::Apple,
        });
        slots.push(slot_craft_3);
        let mut slot_craft_4 = Slot::new(
            (size + size * 1.0 / 8.0) * 2.0,
            scale * 5.0
                - (scale * 24.5 + size * 1.0 / 9.0 + size * (2.0 + 1.0 / 5.0)
                    - (size + size * 1.0 / 9.0)),
            size,
        );
        slot_craft_4.item = Some(Item {
            stack: Default::default(),
            material: Material::Apple,
        });
        slots.push(slot_craft_4);
        let slot_head = Slot::new(
            x_offset,
            y_offset
                + -((6_f64 + 1.0 / 8.0) * (size + size * 1.0 / 8.0) + size + hot_bar_offset * 2.0),
            size,
        ); // 6th slot!
        let slot_chestplate = Slot::new(
            x_offset,
            y_offset
                + -((5_f64 + 1.0 / 8.0) * (size + size * 1.0 / 8.0) + size + hot_bar_offset * 2.0),
            size,
        ); // 7th slot!
        let slot_leggings = Slot::new(
            x_offset,
            y_offset
                + -((4_f64 + 1.0 / 8.0) * (size + size * 1.0 / 8.0) + size + hot_bar_offset * 2.0),
            size,
        ); // 8th slot!
        let slot_boots = Slot::new(
            x_offset,
            y_offset
                + -((3_f64 + 1.0 / 8.0) * (size + size * 1.0 / 8.0) + size + hot_bar_offset * 2.0),
            size,
        ); // 9th slot!
        slots.push(slot_head);
        slots.push(slot_chestplate);
        slots.push(slot_leggings);
        slots.push(slot_boots);
        for y in (0..3).rev() {
            for x in 0..9 {
                slots.push(Slot::new(
                    x_offset + (x as f64) * (size + size * 1.0 / 8.0),
                    y_offset
                        + -((y as f64 + 1.0 / 8.0) * (size + size * 1.0 / 8.0)
                            + size
                            + size * 1.0 / 8.0
                            + hot_bar_offset),
                    size,
                ));
            }
        }
        for i in 0..9 {
            slots.push(Slot::new(
                x_offset + (i as f64) * (size + size * 1.0 / 8.0),
                y_offset,
                size,
            ));
        }
        if version > Version::V1_8 {
            let mut slot = Slot::new(-(scale * 3.0), scale * 5.0 - scale * 18.0, size);
            slot.item = Some(Item {
                stack: Default::default(),
                material: Material::Apple,
            });
            slots.push(slot);
        }
        Self {
            slots,
            dirty: false,
            version,
            hud_context,
        }
    }

    fn update_icons(&mut self, renderer: &Renderer) {
        let scale = Hud::icon_scale(renderer);
        let size = scale * 16.0;
        let x_offset = -(size * 4.5);
        let y_offset = size * 4.18;
        let hot_bar_offset = scale * 4.0;
        let slot_offset = size + size * 1.0 / 8.0;
        let slot_craft_0 = self.slots.get_mut(0).unwrap();
        // slot_craft_0.update_position((size + size * 1.0 / 8.0) * (4.0 + 2.0), scale / 9.0 * 5.0 - (scale / 9.0 * 24.5 + size * 1.0 / 9.0 + size * (2.0 + 1.0 / 5.0) - (size + size * 1.0 / 9.0)), size);
        slot_craft_0.update_position(0.0/*(size + size * 1.0 / 8.0) * 2.5*/, 0.0/*scale / 9.0 * 5.0 - (scale / 9.0 * 24.5 + size * 1.0 / 8.0 + size * (2.0 + 1.0 / 5.0) * 2.0)*/, size); // TODO: The 0th slot isn't rendered, no matter what! FIX THIS!
        let slot_craft_1 = self.slots.get_mut(1).unwrap();
        slot_craft_1.update_position(
            size + size * 1.0 / 8.0,
            scale * 5.0 - (scale * 24.5 + size * 1.0 / 8.0 + size * (2.0 + 1.0 / 5.0)),
            size,
        );
        let slot_craft_2 = self.slots.get_mut(2).unwrap();
        slot_craft_2.update_position(
            (size + size * 1.0 / 8.0) * 2.0,
            scale * 5.0 - (scale * 24.5 + size * 1.0 / 8.0 + size * (2.0 + 1.0 / 5.0)),
            size,
        );
        let slot_craft_3 = self.slots.get_mut(3).unwrap();
        slot_craft_3.update_position(
            size + size * 1.0 / 8.0,
            scale * 5.0
                - (scale * 24.5 + size * 1.0 / 8.0 + size * (2.0 + 1.0 / 5.0) - slot_offset),
            size,
        );
        let slot_craft_4 = self.slots.get_mut(4).unwrap();
        slot_craft_4.update_position(
            (size + size * 1.0 / 8.0) * 2.0,
            scale * 5.0
                - (scale * 24.5 + size * 1.0 / 8.0 + size * (2.0 + 1.0 / 5.0) - slot_offset),
            size,
        );
        let slot_head = self.slots.get_mut(5).unwrap(); // 6th slot!
        slot_head.update_position(
            x_offset,
            y_offset + -((6_f64 + 1.0 / 8.0) * slot_offset + size + hot_bar_offset * 2.0),
            size,
        );
        let slot_chestplate = self.slots.get_mut(6).unwrap(); // 7th slot!
        slot_chestplate.update_position(
            x_offset,
            y_offset + -((5_f64 + 1.0 / 8.0) * slot_offset + size + hot_bar_offset * 2.0),
            size,
        );
        let slot_leggings = self.slots.get_mut(7).unwrap(); // 8th slot!
        slot_leggings.update_position(
            x_offset,
            y_offset + -((4_f64 + 1.0 / 8.0) * slot_offset + size + hot_bar_offset * 2.0),
            size,
        );
        let slot_boots = self.slots.get_mut(8).unwrap(); // 9th slot!
        slot_boots.update_position(
            x_offset,
            y_offset + -((3_f64 + 1.0 / 8.0) * slot_offset + size + hot_bar_offset * 2.0),
            size,
        );
        if self.version > Version::V1_8 {
            let slot = self.slots.get_mut(45).unwrap();
            slot.update_position(-(scale * 3.0), scale * 5.0 - scale * 18.0, size);
        }
        self.dirty = true;
    }

    pub(crate) fn get_raw_slot_mut(&mut self, idx: u16) -> &mut Slot {
        self.slots.get_mut(idx as usize).unwrap()
    }

    pub(crate) fn get_raw_slot(&self, idx: u16) -> &Slot {
        self.slots.get(idx as usize).unwrap()
    }
}

impl Inventory for PlayerInventory {
    fn size(&self) -> u16 {
        self.slots.len() as u16
    }

    fn id(&self) -> i32 {
        -1
    }

    fn name(&self) -> Option<&String> {
        None
    }

    fn get_item(&self, slot: u16) -> Option<Item> {
        self.slots[slot as usize].item.clone()
    }

    fn set_item(&mut self, slot: u16, item: Option<Item>) {
        self.slots[slot as usize].item = item;
        self.dirty = true;
        self.hud_context
            .clone()
            .read()
            .dirty_slots
            .store(true, Ordering::Relaxed);
    }

    fn init(
        &mut self,
        renderer: &mut Renderer,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        inventory_window.elements.push(vec![]);
        let basic_elements = inventory_window.elements.get_mut(1).unwrap();
        let icon_scale = Hud::icon_scale(renderer);
        let image = ui::ImageBuilder::new()
            .texture_coords((0.0 / 256.0, 0.0 / 256.0, 176.0 / 256.0, 166.0 / 256.0))
            .position(0.0, 0.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
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
                .position(-(icon_scale * 3.0), icon_scale * 5.0 - icon_scale * 18.0)
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .size(icon_scale * 18.0, icon_scale * 18.0)
                .texture("minecraft:gui/container/inventory")
                .create(ui_container);
            basic_elements.push(image);
        }
        let scale = icon_scale / 2.0;
        inventory_window.text_elements.push(vec![]);
        let basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();
        let crafting_text = ui::TextBuilder::new()
            .alignment(VAttach::Middle, HAttach::Center)
            .scale_x(scale)
            .scale_y(scale)
            .position(icon_scale * 9.0 * 3.2, -(icon_scale * 9.0 * 7.80))
            .text("Crafting")
            .colour((64, 64, 64, 255))
            .shadow(false)
            .create(ui_container);
        basic_text_elements.push(crafting_text);
        inventory_window.elements.push(vec![]);
        self.update_icons(renderer);
        self.hud_context
            .clone()
            .read()
            .dirty_slots
            .store(true, Ordering::Relaxed);
    }

    fn tick(
        &mut self,
        renderer: &mut Renderer,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        if self.dirty {
            self.dirty = false;
            inventory_window.elements.get_mut(2).unwrap().clear();
            for slot in 0..9 {
                let slot = self.slots.get(slot).unwrap();
                if let Some(item) = &slot.item {
                    inventory_window.draw_item_internally(
                        item,
                        slot.x,
                        slot.y,
                        2,
                        ui_container,
                        renderer,
                        VAttach::Middle,
                    );
                }
            }
            if self.slots.len() == 46 {
                let slot = self.slots.get(45).unwrap();
                if let Some(item) = &slot.item {
                    inventory_window.draw_item_internally(
                        item,
                        slot.x,
                        slot.y,
                        2,
                        ui_container,
                        renderer,
                        VAttach::Middle,
                    );
                }
            }
        }
    }

    fn close(&mut self) {
        // TODO
    }

    fn click_at(&self, _cursor: (u32, u32)) {
        // TODO
    }

    fn resize(
        &mut self,
        _width: u32,
        _height: u32,
        renderer: &mut Renderer,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        self.init(renderer, ui_container, inventory_window);
    }

    fn ty(&self) -> InventoryType {
        InventoryType::Main
    }
}
