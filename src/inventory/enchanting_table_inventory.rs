use crate::inventory::slot_mapping::SlotMapping;
use crate::inventory::{Inventory, InventoryType, Item};
use crate::render::hud::Hud;
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use core::fmt;
use leafish_protocol::protocol::packet;
use leafish_protocol::types::GameMode;
use std::convert::TryFrom;
use std::sync::Arc;

use log::warn;
use parking_lot::RwLock;

use super::Material;

const WINDOW_WIDTH: i32 = 176;
const WINDOW_HEIGHT: i32 = 166;

//textur coords for the buttons
const BUTTON_ACTIVE: (f64, f64, f64, f64) =
    (0.0 / 256.0, 166.0 / 256.0, 108.0 / 256.0, 19.0 / 256.0);
const BUTTON_INACTIVE: (f64, f64, f64, f64) =
    (0.0 / 256.0, 185.0 / 256.0, 108.0 / 256.0, 19.0 / 256.0);
const BUTTON_FOCUSED: (f64, f64, f64, f64) =
    (0.0 / 256.0, 204.0 / 256.0, 108.0 / 256.0, 19.0 / 256.0);

pub struct EnchantmentTableInventory {
    slots: SlotMapping,
    client_state_id: i16,
    id: i32,
    name: String,
    button_data: [EnchantmentButton; 3],
    //if dirty is true we need to redraw the inventory
    dirty: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct EnchantmentButton {
    level_required: Option<u8>,
    enchanmtment_hint: Option<Enchantment>,
    enchantment_level: Option<u8>,
}

impl EnchantmentTableInventory {
    pub fn new(
        renderer: Arc<Renderer>,
        base_slots: Arc<RwLock<SlotMapping>>,
        name: String,
        id: i32,
    ) -> Self {
        let mut slots = SlotMapping::new((WINDOW_WIDTH, WINDOW_HEIGHT));
        slots.set_child(base_slots, (8, 84), (2..38).collect());

        //item slot
        slots.add_slot(0, (15, 47));
        //lapis slot
        slots.add_slot(1, (35, 47));

        slots.update_icons(renderer, (0, 0), None);

        Self {
            slots,
            client_state_id: 0,
            name,
            id,
            button_data: [Default::default(); 3],
            dirty: true,
        }
    }
}

impl Inventory for EnchantmentTableInventory {
    fn size(&self) -> u16 {
        self.slots.size()
    }

    fn handle_property_packet(&mut self, property: i16, value: i16) {
        //the server will send -1 to reset the value
        let option_value = if value == -1 { None } else { Some(value as u8) };

        let update_level_required = |i: usize, this: &mut EnchantmentTableInventory| {
            this.button_data[i] = EnchantmentButton {
                level_required: option_value,
                ..this.button_data[i]
            }
        };
        let update_enchantment_hint = |i: usize, this: &mut EnchantmentTableInventory| {
            this.button_data[i] = EnchantmentButton {
                enchanmtment_hint: Enchantment::try_from(value).ok(),
                ..this.button_data[i]
            }
        };
        let update_enchantment_level = |i: usize, this: &mut EnchantmentTableInventory| {
            this.button_data[i] = EnchantmentButton {
                enchantment_level: option_value,
                ..this.button_data[i]
            }
        };

        match property {
            0 => update_level_required(0, self),
            1 => update_level_required(1, self),
            2 => update_level_required(2, self),
            3 => (), // this is a random seed, usually used for the SGA text
            4 => update_enchantment_hint(0, self),
            5 => update_enchantment_hint(1, self),
            6 => update_enchantment_hint(2, self),
            7 => update_enchantment_level(0, self),
            8 => update_enchantment_level(1, self),
            9 => update_enchantment_level(2, self),
            _ => warn!("the server sent invalid data for the enchanting table"),
        }

        // if there is no level requirement, the server will sometimes send no data,
        // and sometimes send a level requirement of 0 to indicate that
        for i in 0..3 {
            if self.button_data[i].level_required == Some(0) {
                self.button_data[i].level_required = None
            }
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
        self.client_state_id = client_state_id;
        self.dirty = true;
    }

    fn get_item(&self, slot_id: u16) -> Option<Item> {
        self.slots.get_item(slot_id)
    }

    fn set_item(&mut self, slot_id: u16, item: Option<Item>) {
        // TODO: actually lock the slot without sending packet
        // if we attempt to put no lapis into the slot
        let is_lapis_or_none = |item: &Option<Item>| {
            item.is_none() || item.clone().unwrap_or_default().material == Material::LapisLazuli
        };

        if (slot_id == 1 && is_lapis_or_none(&item)) || slot_id != 1 {
            self.slots.set_item(slot_id, item);
            self.dirty = true;
        }
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

        // enchantment table texture
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((0.0 / 256.0, 0.0 / 256.0, 176.0 / 256.0, 166.0 / 256.0))
                .position(top_left_x, top_left_y)
                .alignment(ui::VAttach::Top, ui::HAttach::Left)
                .size(icon_scale * 176.0, icon_scale * 166.0)
                .texture("minecraft:gui/container/enchanting_table")
                .create(ui_container),
        );

        basic_text_elements.push(
            ui::TextBuilder::new()
                .alignment(VAttach::Top, HAttach::Left)
                .scale_x(icon_scale / 2.0)
                .scale_y(icon_scale / 2.0)
                .position(top_left_x + 8.0 * icon_scale, top_left_y + 8.0 * icon_scale)
                .text(if self.name.as_str() == "container.enchant" {
                    "Enchant"
                } else {
                    self.name.as_str()
                })
                .colour((64, 64, 64, 255))
                .shadow(false)
                .create(ui_container),
        );

        let ench_button = ui::ImageBuilder::new()
            .texture_coords(BUTTON_INACTIVE)
            .alignment(ui::VAttach::Top, ui::HAttach::Left)
            .size(icon_scale * 109.0, icon_scale * 20.0)
            .texture("minecraft:gui/container/enchanting_table");

        for i in 0..3 {
            basic_elements.push(
                ench_button
                    .clone()
                    .position(
                        top_left_x + 59.0 * icon_scale,
                        top_left_y + 13.0 * icon_scale + 19.0 * i as f64 * icon_scale,
                    )
                    .create(ui_container),
            );
            basic_elements
                .last()
                .unwrap()
                .borrow_mut()
                .add_hover_func(|this, hover, _| {
                    // this colors the buttons purple if hovered
                    if this.texture_coords == BUTTON_ACTIVE || this.texture_coords == BUTTON_FOCUSED
                    {
                        if hover {
                            this.texture_coords = BUTTON_FOCUSED
                        } else {
                            this.texture_coords = BUTTON_ACTIVE
                        }
                    }
                    true
                });
            let id = self.id() as u8;
            basic_elements
                .last()
                .unwrap()
                .borrow_mut()
                .add_click_func(move |this, game| {
                    if this.texture_coords == BUTTON_FOCUSED {
                        game.server.clone().unwrap().write_packet(
                            packet::play::serverbound::ClickWindowButton { id, button: i },
                        );
                    }
                    true
                })
        }

        let level_requ = ui::ImageBuilder::new()
            .alignment(ui::VAttach::Top, ui::HAttach::Left)
            .size(icon_scale * 13.0, icon_scale * 11.0)
            .draw_index(50)
            .texture("minecraft:gui/container/enchanting_table");

        for i in 0..3 {
            basic_elements.push(
                level_requ
                    .clone()
                    .texture_coords((
                        (2.0 + 16.0 * i as f64) / 256.0,
                        225.0 / 256.0,
                        13.0 / 256.0,
                        11.0 / 256.0,
                    ))
                    .position(
                        top_left_x + 62.0 * icon_scale,
                        top_left_y + 17.0 * icon_scale + 19.0 * i as f64 * icon_scale,
                    )
                    .create(ui_container),
            );
        }

        let button_text = ui::TextBuilder::new()
            .alignment(VAttach::Top, HAttach::Left)
            .scale_x(icon_scale / 2.0)
            .scale_y(icon_scale / 2.0)
            .text("");

        // text that shows cost
        for i in 0..3 {
            basic_text_elements.push(
                button_text
                    .clone()
                    .colour((104, 176, 60, 255))
                    .shadow(true)
                    .position(
                        top_left_x + 79.0 * icon_scale,
                        top_left_y + 23.0 * icon_scale + 19.0 * i as f64 * icon_scale,
                    )
                    .create(ui_container),
            );
        }

        // text that shows enchantment and level
        for i in 0..3 {
            basic_text_elements.push(
                button_text
                    .clone()
                    .colour((80, 80, 80, 255))
                    .shadow(false)
                    .position(
                        top_left_x + 79.0 * icon_scale,
                        top_left_y + 15.0 * icon_scale + 19.0 * i as f64 * icon_scale,
                    )
                    .create(ui_container),
            );
        }

        self.slots.update_icons(renderer, (0, 0), None);
        self.dirty = true;
    }

    fn tick(
        &mut self,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        self.slots.tick(renderer, ui_container, inventory_window, 1);
        if self.dirty {
            for i in 0..3 {
                if self.button_data[i].level_required.is_some()
                    || self.button_data[i].enchantment_level.is_some()
                    || self.button_data[i].enchanmtment_hint.is_some()
                {
                    let basic_elements = inventory_window.elements.get_mut(0).unwrap();
                    let basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();
                    let len_el = basic_elements.len();
                    let len_t_el = basic_text_elements.len();

                    // this just makes the following if statement more readable
                    let is_lapis_enough = self.get_item(1).unwrap_or_default().material
                        == Material::LapisLazuli
                        && self.get_item(1).unwrap_or_default().stack.count >= i as isize + 1;
                    let enough_xp = inventory_window
                        .inventory_context
                        .read()
                        .hud_context
                        .read()
                        .exp_level
                        >= self.button_data[i].level_required.unwrap_or_default() as i32;
                    let is_creative_mode = inventory_window
                        .inventory_context
                        .read()
                        .hud_context
                        .read()
                        .game_mode
                        == GameMode::Creative;

                    // the actual buttons to click on
                    if (is_lapis_enough && enough_xp) || is_creative_mode {
                        basic_elements
                            .get_mut(len_el - (6 - i))
                            .unwrap()
                            .borrow_mut()
                            .texture_coords = BUTTON_ACTIVE
                    } else {
                        basic_elements
                            .get_mut(len_el - (6 - i))
                            .unwrap()
                            .borrow_mut()
                            .texture_coords = BUTTON_INACTIVE
                    }

                    // the level requirement indicators (1, 2, 3)
                    basic_elements
                        .get_mut(len_el - (3 - i))
                        .unwrap()
                        .borrow_mut()
                        .texture_coords = (
                        (2.0 + 16.0 * i as f64) / 256.0,
                        225.0 / 256.0,
                        13.0 / 256.0,
                        11.0 / 256.0,
                    );

                    // name of the enchantment and level
                    basic_text_elements
                        .get_mut(len_t_el - (3 - i))
                        .unwrap()
                        .borrow_mut()
                        .text = if let Some(ench) = self.button_data[i].enchanmtment_hint {
                        // in theory we dont need this, but it catches errors if the server sends
                        // invalid enchantment data
                        format!(
                            "{ench} {}",
                            if let Some(level) = self.button_data[i].enchantment_level {
                                level.to_string()
                            } else {
                                "level unknown".to_string()
                            }
                        )
                    } else {
                        "".to_string()
                    };

                    // cost of the enchantment
                    basic_text_elements
                        .get_mut(len_t_el - (6 - i))
                        .unwrap()
                        .borrow_mut()
                        .text = format!(
                        "lvl required: {}",
                        if let Some(cost) = self.button_data[i].level_required {
                            cost.to_string()
                        } else {
                            "lvl required: unknown".to_string()
                        }
                    );
                } else {
                    // deactivate button and clear text
                    let basic_elements = inventory_window.elements.get_mut(0).unwrap();
                    let basic_text_elements = inventory_window.text_elements.get_mut(0).unwrap();
                    let len_el = basic_elements.len();
                    let len_t_el = basic_text_elements.len();

                    // the enchantment cost indicator
                    basic_elements
                        .get_mut(len_el - (3 - i))
                        .unwrap()
                        .borrow_mut()
                        .texture_coords = (
                        (2.0 + 16.0 * i as f64) / 256.0,
                        241.0 / 256.0,
                        13.0 / 256.0,
                        11.0 / 256.0,
                    );

                    // the buttons
                    basic_elements
                        .get_mut(len_el - (6 - i))
                        .unwrap()
                        .borrow_mut()
                        .texture_coords = BUTTON_INACTIVE;

                    // the required xp levl
                    basic_text_elements
                        .get_mut(len_t_el - (6 - i))
                        .unwrap()
                        .borrow_mut()
                        .text = "".to_string();

                    // name of suggested enchantment
                    basic_text_elements
                        .get_mut(len_t_el - (3 - i))
                        .unwrap()
                        .borrow_mut()
                        .text = "".to_string();
                }
            }
            self.dirty = false;
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
        self.dirty = true;
    }

    fn ty(&self) -> InventoryType {
        InventoryType::EnchantingTable
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
#[allow(dead_code)]
enum Enchantment {
    Protection,
    FireProtection,
    FeatherFalling,
    BlastProtection,
    ProjectileProtection,
    Respiration,
    AquaAffinity,
    Thorns,
    DepthStrider,
    FrostWalker,
    CurseofBinding,
    SoulSpeed,
    Sharpness,
    Smite,
    BaneofArthropods,
    Knockback,
    FireAspect,
    Looting,
    SweepingEdge,
    Efficiency,
    SilkTouch,
    Unbreaking,
    Fortune,
    Power,
    Punch,
    Flame,
    Infinity,
    LuckoftheSea,
    Lure,
    Loyalty,
    Impaling,
    Riptide,
    Channeling,
    Multishot,
    QuickCharge,
    Piercing,
    Mending,
    CurseofVanishing,
}

impl fmt::Display for Enchantment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FireProtection => write!(f, "Fire Protection"),
            Self::FeatherFalling => write!(f, "Feather Falling"),
            Self::BlastProtection => write!(f, "Blast Protection"),
            Self::ProjectileProtection => write!(f, "Projectile Protection"),
            Self::AquaAffinity => write!(f, "Aqua Affinity"),
            Self::DepthStrider => write!(f, "Depth Strider"),
            Self::FrostWalker => write!(f, "Frost Walker"),
            Self::CurseofBinding => write!(f, "Curse of Binding"),
            Self::SoulSpeed => write!(f, "Soul Speed"),
            Self::BaneofArthropods => write!(f, "Bane of Arthropds"),
            Self::FireAspect => write!(f, "Fire Aspect"),
            Self::SweepingEdge => write!(f, "Sweeping Edge"),
            Self::LuckoftheSea => write!(f, "Luck of the Sea"),
            Self::QuickCharge => write!(f, "Quick Charge"),
            Self::CurseofVanishing => write!(f, "Curse of Vanishing"),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl TryFrom<i16> for Enchantment {
    type Error = &'static str;
    fn try_from(value: i16) -> Result<Self, Self::Error> {
        let value = value as u8;
        // minecraft has 37 enchantments, this needs to be updated for new enchantments
        if (0..38).contains(&value) {
            Ok(unsafe { std::mem::transmute::<u8, Enchantment>(value) })
        } else {
            Err("something went wrong")
        }
    }
}
