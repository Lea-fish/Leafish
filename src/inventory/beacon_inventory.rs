use crate::inventory::slot_mapping::SlotMapping;
use crate::inventory::{Inventory, InventoryType, Item};
use crate::protocol::packet;
use crate::protocol::VarInt;
use crate::render::hud::Hud;
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui;
use crate::ui::{Container, HAttach, VAttach};
use log::warn;
use parking_lot::RwLock;
use std::convert::TryFrom;
use std::sync::Arc;

use super::Material;

const WINDOW_WIDTH: i32 = 230;
const WINDOW_HEIGHT: i32 = 219;

const BUTTON_ACTIVE: (f64, f64, f64, f64) = (0.0, 219.0, 22.0, 22.0);
const BUTTON_PRESSED: (f64, f64, f64, f64) = (22.0, 219.0, 22.0, 22.0);
const BUTTON_INACTIVE: (f64, f64, f64, f64) = (44.0, 219.0, 22.0, 22.0);
const BUTTON_FOCUSED: (f64, f64, f64, f64) = (66.0, 219.0, 22.0, 22.0);

pub struct BeaconInventory {
    slots: SlotMapping,
    client_state_id: i16,
    id: i32,
    info: BeaconInfo,
    dirty: bool,
}

struct BeaconInfo {
    power_level: u8,
    effect1: Option<Effect>,
    effect2: Option<Effect>,
}

impl BeaconInventory {
    pub fn new(renderer: Arc<Renderer>, base_slots: Arc<RwLock<SlotMapping>>, id: i32) -> Self {
        let mut slots = SlotMapping::new((WINDOW_WIDTH, WINDOW_HEIGHT));
        slots.set_child(base_slots, (36, 137), (1..37).collect());

        //item slot
        slots.add_slot(0, (136, 110));

        slots.update_icons(renderer, (0, 0), None);

        Self {
            slots,
            client_state_id: 0,
            id,
            info: BeaconInfo {
                power_level: 0,
                effect1: None,
                effect2: None,
            },
            dirty: true,
        }
    }
}

impl Inventory for BeaconInventory {
    fn size(&self) -> u16 {
        self.slots.size()
    }

    fn handle_property_packet(&mut self, property: i16, value: i16) {
        match property {
            0 => self.info.power_level = value as u8,
            1 => self.info.effect1 = Effect::try_from(value).ok(),
            2 => self.info.effect2 = Effect::try_from(value).ok(),
            _ => warn!("the server sent invalid data for the beacon"),
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
        inventory_window.elements.push(vec![]); // Window textures
        inventory_window.elements.push(vec![]); // Item slot
        inventory_window.elements.push(vec![]); // Base slots
        inventory_window.text_elements.push(vec![]);

        let basic_elements = inventory_window.elements.get_mut(0).unwrap();
        let icon_scale = Hud::icon_scale(renderer.clone());

        let top_left_x =
            renderer.screen_data.read().center().0 as f64 - icon_scale * WINDOW_WIDTH as f64 / 2.0;
        let top_left_y =
            renderer.screen_data.read().center().1 as f64 - icon_scale * WINDOW_HEIGHT as f64 / 2.0;

        basic_elements.push(
            ui::ImageBuilder::new()
                .texture_coords((0.0, 0.0, WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64))
                .position(top_left_x, top_left_y)
                .alignment(VAttach::Top, HAttach::Left)
                .size(
                    icon_scale * WINDOW_WIDTH as f64,
                    icon_scale * WINDOW_HEIGHT as f64,
                )
                .texture("minecraft:gui/container/beacon")
                .create(ui_container),
        );

        let mut pos: [(f64, f64, &str); 8] = [
            (53.0, 22.0, "minecraft:mob_effect/speed"),
            (77.0, 22.0, "minecraft:mob_effect/haste"),
            (53.0, 47.0, "minecraft:mob_effect/resistance"),
            (77.0, 47.0, "minecraft:mob_effect/jump_boost"),
            (65.0, 72.0, "minecraft:mob_effect/strength"),
            (144.0, 47.0, "minecraft:mob_effect/regeneration"),
            (168.0, 47.0, ""),
            (164.0, 107.0, ""),
        ];

        for i in 0..8 {
            pos[i].0 = top_left_x + pos[i].0 * icon_scale;
            pos[i].1 = top_left_y + pos[i].1 * icon_scale;
        }
        // buttons
        for i in 0..8 {
            basic_elements.push(
                ui::ImageBuilder::new()
                    .texture("minecraft:gui/container/beacon")
                    .texture_coords(BUTTON_INACTIVE)
                    .position(pos[i].0, pos[i].1)
                    .alignment(VAttach::Top, HAttach::Left)
                    .size(21.0 * icon_scale, 21.0 * icon_scale)
                    .create(ui_container),
            );
            basic_elements
                .last()
                .unwrap()
                .borrow_mut()
                .add_hover_func(|this, hover, _| {
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
            basic_elements
                .get_mut(i + 1)
                .unwrap()
                .borrow_mut()
                .add_click_func(move |this, _| {
                    if this.texture_coords == BUTTON_FOCUSED {
                        this.texture_coords = BUTTON_PRESSED;
                    }
                    true
                })
        }

        // effect images
        for i in 0..7 {
            basic_elements.push(
                ui::ImageBuilder::new()
                    .texture(pos[i].2)
                    .position(pos[i].0 + 2.0 * icon_scale, pos[i].1 + 2.0 * icon_scale)
                    .alignment(VAttach::Top, HAttach::Left)
                    .size(18.0 * icon_scale, 18.0 * icon_scale)
                    .create(ui_container),
            )
        }
        basic_elements.get_mut(15).unwrap().borrow_mut().colour.3 = 0;
        basic_elements.get_mut(7).unwrap().borrow_mut().colour.3 = 0;
        // tick
        basic_elements.push(
            ui::ImageBuilder::new()
                .texture("minecraft:gui/container/beacon")
                .texture_coords((88.0, 219.0, 21.0, 21.0))
                .position(pos[7].0, pos[7].1)
                .alignment(VAttach::Top, HAttach::Left)
                .size(21.0 * icon_scale, 21.0 * icon_scale)
                .create(ui_container),
        );

        let payments = [
            "minecraft:item/netherite_ingot",
            "minecraft:item/emerald",
            "minecraft:item/diamond",
            "minecraft:item/gold_ingot",
            "minecraft:item/iron_ingot",
        ];
        for i in 0..5 {
            basic_elements.push(
                ui::ImageBuilder::new()
                    .texture(payments[i])
                    .position(
                        top_left_x + 20.0 * icon_scale + 22.0 * i as f64 * icon_scale,
                        top_left_y + 108.0 * icon_scale,
                    )
                    .alignment(VAttach::Top, HAttach::Left)
                    .size(16.0 * icon_scale, 16.0 * icon_scale)
                    .create(ui_container),
            )
        }

        self.slots.update_icons(renderer, (0, 0), None);
        self.dirty = true;
    }

    // buttons:
    // 0 = bg texture
    // 1-8 = buttons
    // 9-15 = effect images, last one need set texture
    // 16 = tick
    // 17-21 = accepted payments
    fn tick(
        &mut self,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        self.slots.tick(renderer, ui_container, inventory_window, 1);
        let basic_elements = inventory_window.elements.get_mut(0).unwrap();
        for i in 0..5 {
            if get_button_read(i + 1, &basic_elements).texture_coords == BUTTON_PRESSED
                && get_button_read(i + 9, &basic_elements).texture
                    != get_button_read(15, &basic_elements).texture
            {
                self.dirty = true;
            } else if get_button_read(6, &basic_elements).texture_coords == BUTTON_PRESSED
                && get_button_read(7, &basic_elements).texture_coords == BUTTON_PRESSED
            {
                self.dirty = true;
            }
        }

        // send beacon info packet if payed and effect set
        if get_button_read(8, &basic_elements).texture_coords == BUTTON_PRESSED {
            get_button(8, basic_elements).texture_coords = BUTTON_ACTIVE;
            inventory_window
                .inventory_context
                .write()
                .get_conn()
                .write_packet(packet::play::serverbound::SetBeaconEffect {
                    primary_effect: VarInt(Into::<u8>::into(self.info.effect1.unwrap()) as i32),
                    secondary_effect: if let Some(effect) = self.info.effect2 {
                        VarInt(Into::<u8>::into(effect) as i32)
                    } else {
                        VarInt(-1)
                    },
                })
                .expect("couldnt send beacon set effect packet");
        }

        if self.dirty {
            self.dirty = false;
            let n = self.info.power_level as usize * 2;

            // activate all buttons for the beacon power level
            for i in 0..n.min(7) {
                if get_button_read(i + 1, &basic_elements).texture_coords == BUTTON_INACTIVE {
                    get_button(i + 1, basic_elements).texture_coords = BUTTON_ACTIVE;
                }
            }

            // setup the 2 buttons on the right
            if let Some(effect) = self.info.effect1 {
                if self.info.power_level == 4
                    && get_button_read(7, &basic_elements).texture_coords == BUTTON_INACTIVE
                {
                    get_button(7, basic_elements).texture_coords = BUTTON_ACTIVE;
                }
                get_button(15, basic_elements).texture = effect.get_texture();
                get_button(15, basic_elements).colour.3 = 255;
                get_button(7, basic_elements).colour.3 = 255;
            }
            if let Some(effect) = self.info.effect2 {
                if effect == Effect::Regeneration {
                    get_button(6, basic_elements).texture_coords = BUTTON_PRESSED;
                } else {
                    get_button(7, basic_elements).texture_coords = BUTTON_PRESSED;
                }
            }

            // set pressed button on left to button on right
            for i in 0..n.min(5) {
                let current_btn_image = get_button_read(i + 9, &basic_elements).texture.clone();
                let this_btn_effect = Effect::try_from(match i {
                    0 => 1,
                    1 => 3,
                    2 => 11,
                    3 => 8,
                    4 => 5,
                    _ => -1,
                })
                .ok();

                if get_button_read(i + 9, &basic_elements).texture
                    == get_button_read(15, &basic_elements).texture
                {
                    get_button(i + 1, basic_elements).texture_coords = BUTTON_PRESSED;
                }
                if get_button_read(i + 1, &basic_elements).texture_coords == BUTTON_PRESSED
                    && get_button_read(15, &basic_elements).texture != current_btn_image
                {
                    // if the button is pressed and not already set
                    self.info.effect1 = this_btn_effect;
                    if self.info.power_level == 4 {
                        get_button(7, basic_elements).texture_coords = BUTTON_ACTIVE;
                    }
                    get_button(15, basic_elements).texture = current_btn_image;
                    get_button(15, basic_elements).colour.3 = 255;
                    get_button(7, basic_elements).colour.3 = 255;

                    //reset previously pressed buttons
                    for j in 0..n.min(5) {
                        if j != i {
                            get_button(j + 1, basic_elements).texture_coords = BUTTON_ACTIVE;
                        }
                    }
                }

                // choose effect level 2  or regen
                if self.info.power_level == 4
                    && get_button_read(i + 1, &basic_elements).texture_coords == BUTTON_PRESSED
                {
                    if get_button_read(6, &basic_elements).texture_coords == BUTTON_PRESSED
                        && self.info.effect2 != Some(Effect::Regeneration)
                    {
                        get_button(7, basic_elements).texture_coords = BUTTON_ACTIVE;
                        self.info.effect2 = Some(Effect::Regeneration);
                    } else if get_button_read(7, &basic_elements).texture_coords == BUTTON_PRESSED
                        && self.info.effect2 != this_btn_effect
                    {
                        get_button(6, basic_elements).texture_coords = BUTTON_ACTIVE;
                        self.info.effect2 = this_btn_effect;
                    }
                }

                // make the tick active
                if let Some(item) = self.slots.get_item(0) {
                    if item.material == Material::GoldIngot
                        || item.material == Material::Diamond
                        || item.material == Material::IronIngot
                        || item.material == Material::Emerald
                        || item.material == Material::NetheriteIngot
                    {
                        get_button(8, basic_elements).texture_coords = BUTTON_ACTIVE;
                    } else {
                        get_button(8, basic_elements).texture_coords = BUTTON_INACTIVE;
                    }
                } else {
                    get_button(8, basic_elements).texture_coords = BUTTON_INACTIVE;
                }
            }
        }
    }

    fn ty(&self) -> InventoryType {
        InventoryType::Beacon
    }
}

fn get_button(i: usize, basic_elements: &mut Vec<ui::ImageRef>) -> std::cell::RefMut<ui::Image> {
    basic_elements.get_mut(i).unwrap().borrow_mut()
}

fn get_button_read<'a>(
    i: usize,
    basic_elements: &'a &'a mut Vec<ui::ImageRef>,
) -> std::cell::Ref<'a, ui::Image> {
    basic_elements.get(i).unwrap().borrow()
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
#[allow(dead_code)]
enum Effect {
    Speed,
    Slowness,
    Haste,
    MiningFatigue,
    Strength,
    InstantHealth,
    InstantDamage,
    JumpBoost,
    Nausea,
    Regeneration,
    Resistance,
    FireResistance,
    WaterBreathing,
    Invisibility,
    Blindness,
    NightVision,
    Hunger,
    Weakness,
    Poison,
    Wither,
    HealthBoost,
    Absorption,
    Saturation,
    Glowing,
    Levitation,
    Luck,
    BadLuck,
    SlowFalling,
    ConduitPower,
    DolphinsGrace,
    BadOmen,
    HerooftheVillage,
    Darkness,
}

impl Effect {
    fn get_texture(&self) -> String {
        match self {
            Effect::MiningFatigue => "minecraft:mob_effect/minig_fatigue".to_owned(),
            Effect::InstantHealth => "minecraft:mob_effect/instant_health".to_owned(),
            Effect::InstantDamage => "minecraft:mob_effect/instant_damage".to_owned(),
            Effect::JumpBoost => "minecraft:mob_effect/jump_boost".to_owned(),
            Effect::FireResistance => "minecraft:mob_effect/fire_resistance".to_owned(),
            Effect::WaterBreathing => "minecraft:mob_effect/water_breathing".to_owned(),
            Effect::NightVision => "minecraft:mob_effect/night_vision".to_owned(),
            Effect::HealthBoost => "minecraft:mob_effect/health_boost".to_owned(),
            Effect::BadLuck => "minecraft:mob_effect/bad_luck".to_owned(),
            Effect::SlowFalling => "minecraft:mob_effect/slow_falling".to_owned(),
            Effect::ConduitPower => "minecraft:mob_effect/conduit_power".to_owned(),
            Effect::DolphinsGrace => "minecraft:mob_effect/dolphins_grace".to_owned(),
            Effect::BadOmen => "minecraft:mob_effect/bad_omen".to_owned(),
            Effect::HerooftheVillage => "minecraft:mob_effect/hero_of_the_village".to_owned(),
            _ => format!("minecraft:mob_effect/{:?}", self).to_lowercase(),
        }
    }
}

impl TryFrom<i16> for Effect {
    type Error = &'static str;
    fn try_from(value: i16) -> Result<Self, Self::Error> {
        if (0..34).contains(&value) {
            Ok(unsafe { std::mem::transmute::<u8, Effect>(value as u8 - 1) })
        } else if value == -1 {
            Err("no effect")
        } else {
            Err("tried to set a effect that is unknown")
        }
    }
}

impl Into<u8> for Effect {
    fn into(self) -> u8 {
        unsafe { std::mem::transmute::<Effect, u8>(self) + 1 }
    }
}
