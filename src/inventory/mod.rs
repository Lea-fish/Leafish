pub mod anvil;
pub mod beacon;
pub mod brewing_stand;
pub mod chest;
pub mod crafting_table;
pub mod dropper;
pub mod enchanting_table;
pub mod furnace;
pub mod grindstone;
pub(crate) mod material;
pub mod player_inventory;
pub mod slot_mapping;

use crate::inventory::anvil::AnvilInventory;
use crate::inventory::beacon::BeaconInventory;
use crate::inventory::brewing_stand::BrewingStandInventory;
use crate::inventory::chest::ChestInventory;
use crate::inventory::crafting_table::CraftingTableInventory;
use crate::inventory::dropper::DropperInventory;
use crate::inventory::enchanting_table::EnchantmentTableInventory;
use crate::inventory::furnace::FurnaceInventory;
use crate::inventory::grindstone::GrindStoneInventory;
use crate::inventory::player_inventory::PlayerInventory;
use crate::inventory::slot_mapping::SlotMapping;
use crate::render::hud::{Hud, HudContext};
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::screen::ScreenSystem;
use crate::ui::{self, Container, VAttach};
use leafish_blocks as block;
use leafish_protocol::format::Component;
use leafish_protocol::item::Stack;
use leafish_protocol::protocol::packet::InventoryOperation;
use leafish_protocol::protocol::{packet, Conn};
use log::warn;
use parking_lot::RwLock;
use shared::Version;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub trait Inventory {
    /// The number of item slots in this inventory.
    fn size(&self) -> u16;

    /// The id of this inventory. This is either sent to the client by the open
    /// open window packet, or 0 in the case of the player inventory.
    fn id(&self) -> i32;

    /// Get this inventory's current action number. This is a sequence number
    /// sent with every window click packet to allow the server to determine if
    /// any conflicts occurred.
    fn get_client_state_id(&self) -> i16;

    /// Set this inventory's action number. This is normally only done when the
    /// server sends a confirm transaction packet.
    fn set_client_state_id(&mut self, client_state_id: i16);

    /// Get the item currently stored in a slot.
    fn get_item(&self, slot_id: u16) -> Option<Item>;

    /// Set the new item in a slot, replacing any item previous in the slot.
    fn set_item(&mut self, slot_id: u16, item: Option<Item>);

    /// Find the slot containing this position on the screen.
    fn get_slot(&self, x: f64, y: f64) -> Option<u16>;

    fn init(
        &mut self,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    );

    fn tick(
        &mut self,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    );

    fn resize(
        &mut self,
        _width: u32,
        _height: u32,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        self.init(renderer, ui_container, inventory_window);
    }

    fn ty(&self) -> InventoryType;

    // for handeling the WindowProperty packet in inventorys that need it
    fn handle_property_packet(&mut self, _property: i16, _value: i16) {}
}

pub fn inventory_from_type(
    ty: InventoryType,
    title: Component,
    renderer: &Arc<Renderer>,
    base_slots: Arc<RwLock<SlotMapping>>,
    id: i32,
) -> Option<Arc<RwLock<dyn Inventory + Sync + Send>>> {
    Some(match ty {
        /*InventoryType::Internal => {}
        InventoryType::Main => {}*/
        InventoryType::Chest(rows) => Arc::new(RwLock::new(ChestInventory::new(
            renderer,
            base_slots,
            rows,
            title.to_string(),
            id,
        ))),
        InventoryType::CraftingTable => Arc::new(RwLock::new(CraftingTableInventory::new(
            renderer, base_slots, id,
        ))),

        InventoryType::Dropper => Arc::new(RwLock::new(DropperInventory::new(
            renderer,
            base_slots,
            title.to_string(),
            id,
        ))),
        InventoryType::Hopper => Arc::new(RwLock::new(DropperInventory::new(
            renderer,
            base_slots,
            title.to_string(),
            id,
        ))),
        InventoryType::Furnace => Arc::new(RwLock::new(FurnaceInventory::new(
            renderer, base_slots, ty, id,
        ))),
        InventoryType::Smoker => Arc::new(RwLock::new(FurnaceInventory::new(
            renderer, base_slots, ty, id,
        ))),
        InventoryType::BlastFurnace => Arc::new(RwLock::new(FurnaceInventory::new(
            renderer, base_slots, ty, id,
        ))),
        InventoryType::EnchantingTable => Arc::new(RwLock::new(EnchantmentTableInventory::new(
            renderer,
            base_slots,
            title.to_string(),
            id,
        ))),
        InventoryType::Anvil => {
            Arc::new(RwLock::new(AnvilInventory::new(renderer, base_slots, id)))
        }
        InventoryType::Beacon => {
            Arc::new(RwLock::new(BeaconInventory::new(renderer, base_slots, id)))
        }
        InventoryType::BrewingStand => Arc::new(RwLock::new(BrewingStandInventory::new(
            renderer,
            base_slots,
            title.to_string(),
            id,
        ))),
        InventoryType::Grindstone => Arc::new(RwLock::new(GrindStoneInventory::new(
            renderer, base_slots, id,
        ))),
        /*
        InventoryType::Lectern => {}
        InventoryType::Loom => {}
        InventoryType::Merchant => {}
        InventoryType::ShulkerBox => {}
        InventoryType::SmithingTable => {}
        InventoryType::CartographyTable => {}
        InventoryType::Stonecutter => {}
        InventoryType::Horse => {}*/
        _ => return None,
    })
}

#[derive(Debug)]
pub struct Slot {
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub item: Option<Item>,
    // TODO: Add is valid fn for Anvil, crafting, armor etc.
}

impl Slot {
    pub fn new(x: f64, y: f64, size: f64) -> Self {
        Slot {
            x,
            y,
            size,
            item: None,
        }
    }

    pub fn update_position(&mut self, x: f64, y: f64, size: f64) {
        self.x = x;
        self.y = y;
        self.size = size;
    }

    pub fn is_within(&self, x: f64, y: f64) -> bool {
        self.x <= x && x <= (self.x + self.size) && self.y <= y && y <= (self.y + self.size)
    }
}

pub struct InventoryContext {
    pub cursor: Option<Item>,
    pub hotbar_index: u8,
    pub inventory: Option<Arc<RwLock<dyn Inventory + Send + Sync>>>,
    pub safe_inventory: Option<Arc<RwLock<dyn Inventory + Send + Sync>>>,
    pub has_inv_open: bool,
    pub player_inventory: Arc<RwLock<PlayerInventory>>,
    pub base_slots: Arc<RwLock<SlotMapping>>,
    pub hud_context: Arc<RwLock<HudContext>>,
    mouse_position: Option<(f64, f64)>,
    conn: Arc<RwLock<Option<Conn>>>,
    dirty: bool,
}

impl InventoryContext {
    pub fn get_conn(&self) -> Conn {
        self.conn.write().clone().unwrap()
    }

    pub fn new(
        version: Version,
        renderer: &Arc<Renderer>,
        hud_context: Arc<RwLock<HudContext>>,
        conn: Arc<RwLock<Option<Conn>>>,
    ) -> Self {
        let base_slots = {
            let mut slots = SlotMapping::new((160, 74));
            // Main 9x3 grid
            for x in 0..9 {
                for y in 0..3 {
                    slots.add_slot(x + y * 9, (x as i32 * 18, y as i32 * 18));
                }
            }

            // Hotbar
            for x in 0..9 {
                slots.add_slot(x + 27, (x as i32 * 18, 58));
            }

            Arc::new(RwLock::new(slots))
        };

        Self {
            cursor: None,
            hotbar_index: 0,
            inventory: None,
            safe_inventory: None,
            has_inv_open: false,
            player_inventory: Arc::new(RwLock::new(PlayerInventory::new(
                version,
                renderer,
                base_slots.clone(),
            ))),
            base_slots,
            hud_context,
            mouse_position: None,
            conn,
            dirty: false,
        }
    }

    pub fn open_inventory(
        &mut self,
        inventory: Arc<RwLock<dyn Inventory + Sync + Send>>,
        screen_sys: &Arc<ScreenSystem>,
        self_ref: Arc<RwLock<InventoryContext>>,
        server_forced: bool,
    ) {
        self.try_close_inventory(screen_sys, server_forced);
        screen_sys.add_screen(Box::new(InventoryWindow::new(inventory.clone(), self_ref)));
        self.safe_inventory.replace(inventory.clone());
        self.has_inv_open = true;
    }

    pub fn try_close_inventory(
        &mut self,
        screen_sys: &Arc<ScreenSystem>,
        server_forced: bool,
    ) -> bool {
        if self.has_inv_open {
            self.has_inv_open = false;
            if let Some(inventory) = self.safe_inventory.take() {
                // if the close is server-forced we should not close the inventory ourselves again
                // as that might close an additional inventory if the server opens one right after closing
                // this one
                if !server_forced {
                    let inventory_id = inventory.read().id() as u8;
                    let mut conn = self.conn.write();
                    let conn = conn.as_mut().unwrap();
                    packet::send_close_window(conn, inventory_id).unwrap();
                }
            }
            screen_sys.pop_screen();

            // Closing an inventory causes any item being held to be thrown.
            self.cursor = None;

            return true;
        }
        false
    }

    pub fn draw_cursor(
        &mut self,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    ) {
        if self.dirty {
            self.dirty = false;
            inventory_window.cursor_element.clear();
            inventory_window.text_elements.get_mut(3).unwrap().clear();
            if let Some(item) = &self.cursor {
                if let Some(mouse_position) = &self.mouse_position {
                    let icon_scale = Hud::icon_scale(&renderer);
                    let (x, y) = *mouse_position;
                    // Center the icon on the mouse position
                    let x = x - icon_scale * 8.0;
                    let y = y - icon_scale * 8.0;

                    InventoryWindow::draw_item(
                        item,
                        x,
                        y,
                        &mut inventory_window.cursor_element,
                        inventory_window.text_elements.get_mut(3).unwrap(),
                        ui_container,
                        &renderer,
                        VAttach::Top,
                    );
                }
            }
            if let Some((x, y)) = &self.mouse_position {
                inventory_window.formatted_elements.clear();
                if let Some(item) = self
                    .inventory
                    .as_ref()
                    .map(|inv| {
                        inv.read()
                            .get_slot(*x, *y)
                            .map(|slot| inv.read().get_item(slot))
                            .flatten()
                    })
                    .flatten()
                {
                    let icon_scale = Hud::icon_scale(&renderer);
                    let text =
                        ui::FormattedBuilder::new()
                            .scale_x(icon_scale / 2.25)
                            .scale_y(icon_scale / 2.25)
                            .text(item.stack.meta.display_name().unwrap_or_else(|| {
                                Component::from_str(item.material.name().as_str())
                            }))
                            .position(x + icon_scale * 6.0, y - icon_scale * 9.0)
                            .alignment(VAttach::Top, ui::HAttach::Left)
                            .create(ui_container);
                    inventory_window.formatted_elements.push(text);
                    // TODO: add lore support
                }
            }
        }
    }

    #[allow(clippy::collapsible_else_if)]
    pub fn on_click(&mut self, left: bool, shift: bool) {
        if let Some(inventory) = &self.safe_inventory {
            let mut inventory = inventory.write();

            if let Some((x, y)) = self.mouse_position {
                if let Some(slot) = inventory.get_slot(x, y) {
                    self.dirty = true;
                    let mut item = inventory.get_item(slot);
                    let mut conn = self.conn.write();
                    let conn = conn.as_mut().unwrap();

                    // Send the update to the server
                    packet::send_click_container(
                        conn,
                        inventory.id() as u8,
                        slot as i16,
                        if left {
                            if shift {
                                InventoryOperation::ShiftLeftClick
                            } else {
                                InventoryOperation::LeftClick
                            }
                        } else {
                            if shift {
                                InventoryOperation::ShiftRightClick
                            } else {
                                InventoryOperation::RightClick
                            }
                        },
                        inventory.get_client_state_id() as u16,
                        item.as_ref().map(|i| i.stack.clone()),
                    )
                    .unwrap();

                    // Simulate the operation on the inventory screen.
                    (self.cursor, item) = match (self.cursor.clone(), item) {
                        (Some(mut cursor), Some(mut item)) => {
                            // Merge the cursor into a slot stack of the same
                            // material.
                            if item.is_stackable(&cursor) {
                                if left {
                                    let max = item.material.get_stack_size(conn.get_version());
                                    let total = (item.stack.count + cursor.stack.count) as u8;
                                    item.stack.count = total.min(max) as isize;

                                    if total > max {
                                        cursor.stack.count = (total - max) as isize;
                                        (Some(cursor), Some(item))
                                    } else {
                                        (None, Some(item))
                                    }
                                } else {
                                    if item.stack.count
                                        >= item.material.get_stack_size(conn.get_version()) as isize
                                    {
                                        (Some(cursor), Some(item))
                                    } else {
                                        item.stack.count += 1;
                                        if cursor.stack.count <= 1 {
                                            (None, Some(item))
                                        } else {
                                            cursor.stack.count -= 1;
                                            (Some(cursor), Some(item))
                                        }
                                    }
                                }
                            } else {
                                (Some(item), Some(cursor))
                            }
                        }
                        (Some(cursor), None) => {
                            if !left {
                                let mut item = cursor.clone();
                                item.stack.count = 1;
                                let mut cursor = cursor.clone();
                                cursor.stack.count -= 1;
                                (
                                    if cursor.stack.count > 0 {
                                        Some(cursor)
                                    } else {
                                        None
                                    },
                                    Some(item),
                                )
                            } else {
                                (None, Some(cursor))
                            }
                        }
                        (None, Some(mut item)) => {
                            if !left && item.stack.count > 1 {
                                let mut cursor = item.clone();
                                // TODO: this is a stable version of: div_ceil(2) - once this is stabilized, use it instead
                                cursor.stack.count =
                                    (item.stack.count / 2) + (item.stack.count % 2);
                                item.stack.count -= cursor.stack.count;
                                (Some(cursor), Some(item))
                            } else {
                                (Some(item), None)
                            }
                        }
                        (None, None) => (None, None),
                    };
                    inventory.set_item(slot, item);
                    self.hud_context
                        .write()
                        .dirty_slots
                        .store(true, Ordering::Relaxed);
                } else if let Some(cursor) = self.cursor.take() {
                    // when right clicking we don't drop the whole stack but only one item of the stack
                    if !left {
                        let mut cursor = cursor.clone();
                        cursor.stack.count -= 1;
                        self.cursor = Some(cursor);
                    }
                    self.dirty = true;
                    self.hud_context
                        .write()
                        .dirty_slots
                        .store(true, Ordering::Relaxed);
                    let mut conn = self.conn.write();
                    let conn = conn.as_mut().unwrap();

                    // Send the update to the server
                    packet::send_click_container(
                        conn,
                        inventory.id() as u8,
                        -999,
                        if left {
                            InventoryOperation::LeftClickOutside
                        } else {
                            InventoryOperation::RightClickOutside
                        },
                        inventory.get_client_state_id() as u16,
                        Some(cursor.stack),
                    )
                    .unwrap();
                }
            }
        }
    }

    pub fn on_cursor_moved(&mut self, x: f64, y: f64) {
        self.mouse_position = Some((x, y));
        self.dirty = true;
    }

    pub fn set_cursor(&mut self, cursor: Option<Item>) {
        self.cursor = cursor;
        self.dirty = true;
    }

    pub fn on_confirm_transaction(&self, id: u8, action_number: i16, _accepted: bool) {
        if id as i32 == self.player_inventory.read().id() {
            self.player_inventory
                .write()
                .set_client_state_id(action_number);
        } else if let Some(inventory) = &self.safe_inventory {
            let mut inventory = inventory.write();
            if id as i32 != inventory.id() {
                warn!(
                    "Expected inventory id {}, but instead got {id}",
                    inventory.id()
                );
                return;
            }

            inventory.set_client_state_id(action_number);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum InventoryType {
    Internal, // For internal use only.
    Main,
    Chest(u8), // rows
    Dropper,   // Dropper and Dispenser
    Anvil,
    Beacon,
    BlastFurnace,
    BrewingStand,
    CraftingTable,
    EnchantingTable,
    Furnace,
    Grindstone,
    Hopper,
    Lectern,
    Loom,
    Merchant,
    ShulkerBox,
    SmithingTable,
    Smoker,
    CartographyTable,
    Stonecutter,
    Horse,
}

impl InventoryType {
    /// Lookup a window type based on the inventory type strings used in 1.14+.
    pub fn from_id(version: Version, mut id: i32) -> Option<Self> {
        // Before version 1.16, smithing tables didn't have a GUI.
        if version < Version::V1_16 && id > 19 {
            id += 1;
        }

        Some(match id {
            // General-purpose n-row inventory. Used by chest, large chest,
            // minecart with chest, ender chest, and barrel
            0..=5 => InventoryType::Chest((1 + id) as u8),
            // Used by dispenser or dropper
            6 => InventoryType::Dropper,
            7 => InventoryType::Anvil,
            8 => InventoryType::Beacon,
            9 => InventoryType::BlastFurnace,
            10 => InventoryType::BrewingStand,
            11 => InventoryType::CraftingTable,
            12 => InventoryType::EnchantingTable,
            13 => InventoryType::Furnace,
            14 => InventoryType::Grindstone,
            // Used by hopper or minecart with hopper
            15 => InventoryType::Hopper,
            16 => InventoryType::Lectern,
            17 => InventoryType::Loom,
            18 => InventoryType::Merchant,
            19 => InventoryType::ShulkerBox,
            20 => InventoryType::SmithingTable,
            21 => InventoryType::Smoker,
            22 => InventoryType::CartographyTable,
            23 => InventoryType::Stonecutter,
            _ => {
                warn!("Unhandled inventory type {id}");
                return None;
            }
        })
    }

    /// Lookup a window type based on the inventory type strings used between
    /// 1.8 and 1.13.
    pub fn from_name(name: &str, slot_count: u8) -> Option<Self> {
        Some(match name {
            "minecraft:anvil" => InventoryType::Anvil,
            "minecraft:beacon" => InventoryType::Beacon,
            "minecraft:brewing_stand" => InventoryType::BrewingStand,
            "minecraft:chest" | "minecraft:container" => {
                if slot_count % 9 != 0 {
                    warn!("Chest slot count of {slot_count} wasn't divisible by 9");
                    return None;
                }
                InventoryType::Chest(slot_count / 9)
            }
            "minecraft:crafting_table" => InventoryType::CraftingTable,
            "minecraft:dispenser" => InventoryType::Dropper,
            "minecraft:dropper" => InventoryType::Dropper,
            "minecraft:enchanting_table" => InventoryType::EnchantingTable,
            "minecraft:furnace" => InventoryType::Furnace,
            "minecraft:hopper" => InventoryType::Hopper,
            "minecraft:shulker_box" => InventoryType::ShulkerBox,
            _ => {
                warn!("Unhandled inventory type {name}");
                return None;
            }
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Item {
    pub stack: Stack,
    pub material: Material,
}

impl Item {
    /// Check if this item stack matches another, allowing these to be merged
    /// on an inventory slot.
    pub fn is_stackable(&self, other: &Item) -> bool {
        self.material == other.material && self.stack.damage == other.stack.damage
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Material {
    #[default]
    Air, // 1.7.10 (id: 0, stack: 0)| 1.13 (id: 9648)
    Stone,                           // 1.7.10 (id: 1)| 1.13 (id: 22948)
    Grass,                           // 1.7.10 (id: 2)| 1.13 (id: 6155)
    Dirt,                            // 1.7.10 (id: 3)| 1.13 (id: 10580)
    Cobblestone,                     // 1.7.10 (id: 4)| 1.13 (id: 32147)
    Wood,                            // 1.7.10 (id: 5, stack: 0)
    Sapling,                         // 1.7.10 (id: 6, stack: 0)
    Bedrock,                         // 1.7.10 (id: 7)| 1.13 (id: 23130)
    Water,                           // 1.7.10 (id: 8, stack: 0)
    StationaryWater,                 // 1.7.10 (id: 9, stack: 0)
    Lava,                            // 1.7.10 (id: 10, stack: 0)
    StationaryLava,                  // 1.7.10 (id: 11, stack: 0)
    Sand,                            // 1.7.10 (id: 12)| 1.13 (id: 11542)
    Gravel,                          // 1.7.10 (id: 13)| 1.13 (id: 7804)
    GoldOre,                         // 1.7.10 (id: 14)| 1.13 (id: 32625)
    IronOre,                         // 1.7.10 (id: 15)| 1.13 (id: 19834)
    CoalOre,                         // 1.7.10 (id: 16)| 1.13 (id: 30965)
    Log,                             // 1.7.10 (id: 17, stack: 0)
    Leaves,                          // 1.7.10 (id: 18, stack: 0)
    Sponge,                          // 1.7.10 (id: 19)| 1.13 (id: 15860)
    Glass,                           // 1.7.10 (id: 20)| 1.13 (id: 6195)
    LapisOre,                        // 1.7.10 (id: 21)| 1.13 (id: 22934)
    LapisBlock,                      // 1.7.10 (id: 22)| 1.13 (id: 14485)
    Dispenser,                       // 1.7.10 (id: 23, stack: 0)| 1.13 (id: 20871)
    Sandstone,                       // 1.7.10 (id: 24, stack: 0)| 1.13 (id: 13141)
    NoteBlock,                       // 1.7.10 (id: 25)| 1.13 (id: 20979)
    BedBlock,                        // 1.7.10 (id: 26, stack: 0)
    PoweredRail,                     // 1.7.10 (id: 27, stack: 0)| 1.13 (id: 11064)
    DetectorRail,                    // 1.7.10 (id: 28, stack: 0)| 1.13 (id: 13475)
    PistonStickyBase,                // 1.7.10 (id: 29, stack: 0)
    Web,                             // 1.7.10 (id: 30)
    LongGrass,                       // 1.7.10 (id: 31, stack: 0)
    DeadBush,                        // 1.7.10 (id: 32)| 1.13 (id: 22888)
    PistonBase,                      // 1.7.10 (id: 33, stack: 0)
    PistonExtension,                 // 1.7.10 (id: 34, stack: 0)
    Wool,                            // 1.7.10 (id: 35, stack: 0)
    PistonMovingPiece,               // 1.7.10 (id: 36)
    YellowFlower,                    // 1.7.10 (id: 37)
    RedRose,                         // 1.7.10 (id: 38)
    BrownMushroom,                   // 1.7.10 (id: 39)| 1.13 (id: 9665)
    RedMushroom,                     // 1.7.10 (id: 40)| 1.13 (id: 19728)
    GoldBlock,                       // 1.7.10 (id: 41)| 1.13 (id: 27392)
    IronBlock,                       // 1.7.10 (id: 42)| 1.13 (id: 24754)
    DoubleStep,                      // 1.7.10 (id: 43, stack: 0)
    Step,                            // 1.7.10 (id: 44, stack: 0)
    Brick,                           // 1.7.10 (id: 45)| 1.13 (id: 6820)
    Tnt,                             // 1.7.10 (id: 46)| 1.13 (id: 7896)
    Bookshelf,                       // 1.7.10 (id: 47)| 1.13 (id: 10069)
    MossyCobblestone,                // 1.7.10 (id: 48)| 1.13 (id: 21900)
    Obsidian,                        // 1.7.10 (id: 49)| 1.13 (id: 32723)
    Torch,                           // 1.7.10 (id: 50, stack: 0)| 1.13 (id: 6063)
    Fire,                            // 1.7.10 (id: 51)| 1.13 (id: 16396)
    MobSpawner,                      // 1.7.10 (id: 52)
    WoodStairs,                      // 1.7.10 (id: 53, stack: 0)
    Chest,                           // 1.7.10 (id: 54, stack: 0)| 1.13 (id: 22969)
    RedstoneWire,                    // 1.7.10 (id: 55, stack: 0)| 1.13 (id: 25984)
    DiamondOre,                      // 1.7.10 (id: 56)| 1.13 (id: 9292)
    DiamondBlock,                    // 1.7.10 (id: 57)| 1.13 (id: 5944)
    Workbench,                       // 1.7.10 (id: 58)
    Crops,                           // 1.7.10 (id: 59, stack: 0)
    Soil,                            // 1.7.10 (id: 60, stack: 0)
    Furnace,                         // 1.7.10 (id: 61, stack: 0)| 1.13 (id: 8133)
    BurningFurnace,                  // 1.7.10 (id: 62, stack: 0)
    SignPost,                        // 1.7.10 (id: 63, stack: 64, durability: 0 )
    WoodenDoor,                      // 1.7.10 (id: 64, stack: 0)
    Ladder,                          // 1.7.10 (id: 65, stack: 0)| 1.13 (id: 23599)
    Rails,                           // 1.7.10 (id: 66, stack: 0)
    CobblestoneStairs,               // 1.7.10 (id: 67, stack: 0)| 1.13 (id: 24715)
    WallSign,      // 1.7.10(id; 68, stack: 64, durability: 0 )| 1.13 (id: 10644,stack: 0)
    Lever,         // 1.7.10 (id: 69, stack: 0)| 1.13 (id: 15319)
    StonePlate,    // 1.7.10 (id: 70, stack: 0)
    IronDoorBlock, // 1.7.10 (id: 71, stack: 0)
    WoodPlate,     // 1.7.10 (id: 72, stack: 0)
    RedstoneOre,   // 1.7.10 (id: 73)| 1.13 (id: 10887)
    GlowingRedstoneOre, // 1.7.10 (id: 74)
    RedstoneTorchOff, // 1.7.10 (id: 75, stack: 0)
    RedstoneTorchOn, // 1.7.10 (id: 76, stack: 0)
    StoneButton,   // 1.7.10 (id: 77, stack: 0)| 1.13 (id: 12279)
    Snow,          // 1.7.10 (id: 78)| 1.13 (id: 14146)
    Ice,           // 1.7.10 (id: 79)| 1.13 (id: 30428)
    SnowBlock,     // 1.7.10 (id: 80)| 1.13 (id: 19913)
    Cactus,        // 1.7.10 (id: 81, stack: 0)| 1.13 (id: 12191)| 1.16.1 (id: 28478)
    Clay,          // 1.7.10 (id: 82)| 1.13 (id: 27880)
    SugarCaneBlock, // 1.7.10 (id: 83, stack: 0)
    Jukebox,       // 1.7.10 (id: 84)| 1.13 (id: 19264)
    Fence,         // 1.7.10 (id: 85)
    Pumpkin,       // 1.7.10 (id: 86, stack: 0)| 1.13 (id: 19170)
    Netherrack,    // 1.7.10 (id: 87)| 1.13 (id: 23425)
    SoulSand,      // 1.7.10 (id: 88)| 1.13 (id: 16841)
    Glowstone,     // 1.7.10 (id: 89)| 1.13 (id: 32713)
    Portal,        // 1.7.10 (id: 90)
    JackOLantern,  // 1.7.10 (id: 91, stack: 0)| 1.13 (id: 31612)| 1.16.2 (id: 13758)
    CakeBlock,     // 1.7.10(id; 92, stack: 64, durability: 0 )
    DiodeBlockOff, // 1.7.10 (id: 93, stack: 0)
    DiodeBlockOn,  // 1.7.10 (id: 94, stack: 0)
    LockedChest,   // 1.7.10 (id: 95)
    StainedGlass,  // 1.7.10 (id: 95)
    TrapDoor,      // 1.7.10 (id: 96, stack: 0)
    MonsterEggs,   // 1.7.10 (id: 97, stack: 0)
    SmoothBrick,   // 1.7.10 (id: 98, stack: 0)
    HugeMushroom1, // 1.7.10 (id: 99, stack: 0)
    HugeMushroom2, // 1.7.10 (id: 100, stack: 0)
    IronFence,     // 1.7.10 (id: 101)
    ThinGlass,     // 1.7.10 (id: 102)
    MelonBlock,    // 1.7.10 (id: 103)
    PumpkinStem,   // 1.7.10 (id: 104, stack: 0)| 1.13 (id: 19021)
    MelonStem,     // 1.7.10 (id: 105, stack: 0)| 1.13 (id: 8247)
    Vine,          // 1.7.10 (id: 106, stack: 0)| 1.13 (id: 14564)
    FenceGate,     // 1.7.10 (id: 107, stack: 0)
    BrickStairs,   // 1.7.10 (id: 108, stack: 0)| 1.13 (id: 21534)
    SmoothStairs,  // 1.7.10 (id: 109, stack: 0)
    Mycel,         // 1.7.10 (id: 110)
    WaterLily,     // 1.7.10 (id: 111)
    NetherBrick,   // 1.7.10 (id: 112)| 1.13 (id: 19996)
    NetherFence,   // 1.7.10 (id: 113)
    NetherBrickStairs, // 1.7.10 (id: 114, stack: 0)| 1.13 (id: 12085)
    NetherWarts,   // 1.7.10 (id: 115, stack: 0)
    EnchantmentTable, // 1.7.10 (id: 116)
    BrewingStand,  // 1.7.10 (id: 117, stack: 0)| 1.13 (id: 14539)
    Cauldron,      // 1.7.10 (id: 118, stack: 0)| 1.13 (id: 26531)| 1.17 (id: 26531)
    EnderPortal,   // 1.7.10 (id: 119)
    EnderPortalFrame, // 1.7.10 (id: 120)
    EnderStone,    // 1.7.10 (id: 121)
    DragonEgg,     // 1.7.10 (id: 122)| 1.13 (id: 29946)
    RedstoneLampOff, // 1.7.10 (id: 123)
    RedstoneLampOn, // 1.7.10 (id: 124)
    WoodDoubleStep, // 1.7.10 (id: 125, stack: 0)
    WoodStep,      // 1.7.10 (id: 126, stack: 0)
    Cocoa,         // 1.7.10 (id: 127, stack: 0)| 1.13 (id: 29709)
    SandstoneStairs, // 1.7.10 (id: 128, stack: 0)| 1.13 (id: 18474)
    EmeraldOre,    // 1.7.10 (id: 129)| 1.13 (id: 16630)
    EnderChest,    // 1.7.10 (id: 130, stack: 0)| 1.13 (id: 32349)
    TripwireHook,  // 1.7.10 (id: 131, stack: 0)| 1.13 (id: 8130)
    Tripwire,      // 1.7.10 (id: 132, stack: 0)| 1.13 (id: 8810)
    EmeraldBlock,  // 1.7.10 (id: 133)| 1.13 (id: 9914)
    SpruceWoodStairs, // 1.7.10 (id: 134, stack: 0)
    BirchWoodStairs, // 1.7.10 (id: 135, stack: 0)
    JungleWoodStairs, // 1.7.10 (id: 136, stack: 0)
    Command,       // 1.7.10 (id: 137, stack: 0)
    Beacon,        // 1.7.10 (id: 138)| 1.13 (id: 6608)
    CobbleWall,    // 1.7.10 (id: 139)
    FlowerPot,     // 1.7.10 (id: 140, stack: 0)| 1.13 (id: 30567)
    Carrot,        // 1.7.10 (id: 141)| 1.13 (id: 22824)
    Potato,        // 1.7.10 (id: 142)| 1.13 (id: 21088)
    WoodButton,    // 1.7.10 (id: 143, stack: 0)
    Skull,         // 1.7.10 (id: 144, stack: 0)
    Anvil,         // 1.7.10 (id: 145)| 1.13 (id: 18718)
    TrappedChest,  // 1.7.10 (id: 146)| 1.13 (id: 18970)
    GoldPlate,     // 1.7.10 (id: 147)
    IronPlate,     // 1.7.10 (id: 148)
    RedstoneComparatorOff, // 1.7.10 (id: 149)
    RedstoneComparatorOn, // 1.7.10 (id: 150)
    DaylightDetector, // 1.7.10 (id: 151)| 1.13 (id: 8864)
    RedstoneBlock, // 1.7.10 (id: 152)| 1.13 (id: 19496)
    QuartzOre,     // 1.7.10 (id: 153)
    Hopper,        // 1.7.10 (id: 154)| 1.13 (id: 31974)
    QuartzBlock,   // 1.7.10 (id: 155)| 1.13 (id: 11987)
    QuartzStairs,  // 1.7.10 (id: 156, stack: 0)| 1.13 (id: 24079)
    ActivatorRail, // 1.7.10 (id: 157, stack: 0)
    Dropper,       // 1.7.10 (id: 158, stack: 0)| 1.13 (id: 31273)
    StainedClay,   // 1.7.10 (id: 159)
    StainedGlassPane, // 1.7.10 (id: 160)
    Leaves2,       // 1.7.10 (id: 161)
    Log2,          // 1.7.10 (id: 162)
    AcaciaStairs,  // 1.7.10 (id: 163, stack: 0)| 1.13 (id: 17453)
    DarkOakStairs, // 1.7.10 (id: 164, stack: 0)| 1.13 (id: 22921)
    HayBlock,      // 1.7.10 (id: 170)| 1.13 (id: 17461)
    Carpet,        // 1.7.10 (id: 171)
    HardClay,      // 1.7.10 (id: 172)
    CoalBlock,     // 1.7.10 (id: 173)| 1.13 (id: 27968)
    PackedIce,     // 1.7.10 (id: 174)| 1.13 (id: 28993)
    DoublePlant,   // 1.7.10 (id: 175)
    IronSpade,     // 1.7.10(id; 256, stack: 1, durability: 250 )
    IronPickaxe,   // 1.7.10(id; 257, stack: 1, durability: 250 )| 1.13 (id: 8842)
    IronAxe,       // 1.7.10(id; 258, stack: 1, durability: 250 )| 1.13 (id: 15894)
    FlintAndSteel, // 1.7.10(id; 259, stack: 1, durability: 64 )| 1.13 (id: 28620)
    Apple,         // 1.7.10 (id: 260)| 1.13 (id: 7720)
    Bow,           // 1.7.10(id; 261, stack: 1, durability: 384 )| 1.13 (id: 8745)
    Arrow,         // 1.7.10 (id: 262)| 1.13 (id: 31091)
    Coal,          // 1.7.10 (id: 263, stack: 0)| 1.13 (id: 29067)
    Diamond,       // 1.7.10 (id: 264)| 1.13 (id: 20865)
    IronIngot,     // 1.7.10 (id: 265)| 1.13 (id: 24895)
    GoldIngot,     // 1.7.10 (id: 266)| 1.13 (id: 28927)
    IronSword,     // 1.7.10(id; 267, stack: 1, durability: 250 )| 1.13 (id: 10904)
    WoodSword,     // 1.7.10(id; 268, stack: 1, durability: 59 )
    WoodSpade,     // 1.7.10(id; 269, stack: 1, durability: 59 )
    WoodPickaxe,   // 1.7.10(id; 270, stack: 1, durability: 59 )
    WoodAxe,       // 1.7.10(id; 271, stack: 1, durability: 59 )
    StoneSword,    // 1.7.10(id; 272, stack: 1, durability: 131 )| 1.13 (id: 25084)
    StoneSpade,    // 1.7.10(id; 273, stack: 1, durability: 131 )
    StonePickaxe,  // 1.7.10(id; 274, stack: 1, durability: 131 )| 1.13 (id: 14611)
    StoneAxe,      // 1.7.10(id; 275, stack: 1, durability: 131 )| 1.13 (id: 6338)
    DiamondSword,  // 1.7.10(id; 276, stack: 1, durability: 1561 )| 1.13 (id: 27707)
    DiamondSpade,  // 1.7.10(id; 277, stack: 1, durability: 1561 )
    DiamondPickaxe, // 1.7.10(id; 278, stack: 1, durability: 1561 )| 1.13 (id: 24291)
    DiamondAxe,    // 1.7.10(id; 279, stack: 1, durability: 1561 )| 1.13 (id: 27277)
    Stick,         // 1.7.10 (id: 280)| 1.13 (id: 9773)
    Bowl,          // 1.7.10 (id: 281)| 1.13 (id: 32661)
    MushroomSoup,  // 1.7.10 (id: 282, stack: 1)
    GoldSword,     // 1.7.10(id; 283, stack: 1, durability: 32 )
    GoldSpade,     // 1.7.10(id; 284, stack: 1, durability: 32 )
    GoldPickaxe,   // 1.7.10(id; 285, stack: 1, durability: 32 )
    GoldAxe,       // 1.7.10(id; 286, stack: 1, durability: 32 )
    String,        // 1.7.10 (id: 287)| 1.13 (id: 12806)
    Feather,       // 1.7.10 (id: 288)| 1.13 (id: 30548)
    Sulphur,       // 1.7.10 (id: 289)
    WoodHoe,       // 1.7.10(id; 290, stack: 1, durability: 59 )
    StoneHoe,      // 1.7.10(id; 291, stack: 1, durability: 131 )| 1.13 (id: 22855)
    IronHoe,       // 1.7.10(id; 292, stack: 1, durability: 250 )| 1.13 (id: 11339)
    DiamondHoe,    // 1.7.10(id; 293, stack: 1, durability: 1561 )| 1.13 (id: 24050)
    GoldHoe,       // 1.7.10(id; 294, stack: 1, durability: 32 )
    Seeds,         // 1.7.10 (id: 295)
    Wheat,         // 1.7.10 (id: 296)| 1.13 (id: 27709)
    Bread,         // 1.7.10 (id: 297)| 1.13 (id: 32049)
    LeatherHelmet, // 1.7.10(id; 298, stack: 1, durability: 55 )| 1.13 (id: 11624)
    LeatherChestplate, // 1.7.10(id; 299, stack: 1, durability: 80 )| 1.13 (id: 29275)
    LeatherLeggings, // 1.7.10(id; 300, stack: 1, durability: 75 )| 1.13 (id: 28210)
    LeatherBoots,  // 1.7.10(id; 301, stack: 1, durability: 65 )| 1.13 (id: 15282)
    ChainmailHelmet, // 1.7.10(id; 302, stack: 1, durability: 165 )| 1.13 (id: 26114)
    ChainmailChestplate, // 1.7.10(id; 303, stack: 1, durability: 240 )| 1.13 (id: 23602)
    ChainmailLeggings, // 1.7.10(id; 304, stack: 1, durability: 225 )| 1.13 (id: 19087)
    ChainmailBoots, // 1.7.10(id; 305, stack: 1, durability: 195 )| 1.13 (id: 17953)
    IronHelmet,    // 1.7.10(id; 306, stack: 1, durability: 165 )| 1.13 (id: 12025)
    IronChestplate, // 1.7.10(id; 307, stack: 1, durability: 240 )| 1.13 (id: 28112)
    IronLeggings,  // 1.7.10(id; 308, stack: 1, durability: 225 )| 1.13 (id: 18951)
    IronBoots,     // 1.7.10(id; 309, stack: 1, durability: 195 )| 1.13 (id: 8531)
    DiamondHelmet, // 1.7.10(id; 310, stack: 1, durability: 363 )| 1.13 (id: 10755)
    DiamondChestplate, // 1.7.10(id; 311, stack: 1, durability: 528 )| 1.13 (id: 32099)
    DiamondLeggings, // 1.7.10(id; 312, stack: 1, durability: 495 )| 1.13 (id: 11202)| 1.17 (id: 26500)
    DiamondBoots,    // 1.7.10(id; 313, stack: 1, durability: 429 )| 1.13 (id: 16522)
    GoldHelmet,      // 1.7.10(id; 314, stack: 1, durability: 77 )
    GoldChestplate,  // 1.7.10(id; 315, stack: 1, durability: 112 )
    GoldLeggings,    // 1.7.10(id; 316, stack: 1, durability: 105 )
    GoldBoots,       // 1.7.10(id; 317, stack: 1, durability: 91 )
    Flint,           // 1.7.10 (id: 318)| 1.13 (id: 23596)
    Pork,            // 1.7.10 (id: 319)
    GrilledPork,     // 1.7.10 (id: 320)
    Painting,        // 1.7.10 (id: 321)| 1.13 (id: 23945)
    GoldenApple,     // 1.7.10 (id: 322)| 1.13 (id: 27732)
    Sign,            // 1.7.10 (id: 323, stack: 16)| 1.13 (id: 16918)
    WoodDoor,        // 1.7.10 (id: 324, stack: 1)| 1.8 (stack: 64)
    Bucket,          // 1.7.10 (id: 325, stack: 16)| 1.13 (id: 15215)
    WaterBucket,     // 1.7.10 (id: 326, stack: 1)| 1.13 (id: 8802)
    LavaBucket,      // 1.7.10 (id: 327, stack: 1)| 1.13 (id: 9228)
    Minecart,        // 1.7.10 (id: 328, stack: 1)| 1.13 (id: 14352)
    Saddle,          // 1.7.10 (id: 329, stack: 1)| 1.13 (id: 30206)
    IronDoor,        // 1.7.10 (id: 330, stack: 1)| 1.8 (stack: 64)| 1.13 (id: 4788,stack: 0)
    Redstone,        // 1.7.10 (id: 331)| 1.13 (id: 11233)
    SnowBall,        // 1.7.10 (id: 332, stack: 16)| 1.13 (id: 19487)
    Boat,            // 1.7.10 (id: 333, stack: 1)
    Leather,         // 1.7.10 (id: 334)| 1.13 (id: 16414)
    MilkBucket,      // 1.7.10 (id: 335, stack: 1)| 1.13 (id: 9680)
    ClayBrick,       // 1.7.10 (id: 336)
    ClayBall,        // 1.7.10 (id: 337)| 1.13 (id: 24603)
    SugarCane,       // 1.7.10 (id: 338)| 1.13 (id: 7726)
    Paper,           // 1.7.10 (id: 339)| 1.13 (id: 9923)
    Book,            // 1.7.10 (id: 340)| 1.13 (id: 23097)
    SlimeBall,       // 1.7.10 (id: 341)| 1.13 (id: 5242)
    StorageMinecart, // 1.7.10 (id: 342, stack: 1)
    PoweredMinecart, // 1.7.10 (id: 343, stack: 1)
    Egg,             // 1.7.10 (id: 344, stack: 16)| 1.13 (id: 21603)
    Compass,         // 1.7.10 (id: 345)| 1.13 (id: 24139)
    FishingRod,      // 1.7.10(id; 346, stack: 1, durability: 64 )| 1.13 (id: 4167)
    Watch,           // 1.7.10 (id: 347)
    GlowstoneDust,   // 1.7.10 (id: 348)| 1.13 (id: 6665)
    RawFish,         // 1.7.10 (id: 349)
    CookedFish,      // 1.7.10 (id: 350)
    InkSack,         // 1.7.10 (id: 351, stack: 0)
    Bone,            // 1.7.10 (id: 352)| 1.13 (id: 5686)
    Sugar,           // 1.7.10 (id: 353)| 1.13 (id: 30638)
    Cake,            // 1.7.10 (id: 354, stack: 1)| 1.13 (id: 27048)
    Bed,             // 1.7.10 (id: 355, stack: 1)
    Diode,           // 1.7.10 (id: 356)
    Cookie,          // 1.7.10 (id: 357)| 1.13 (id: 27431)
    Map,             // 1.7.10 (id: 358, stack: 0)| 1.13 (id: 21655)
    Shears,          // 1.7.10(id; 359, stack: 1, durability: 238 )| 1.13 (id: 27971)
    Melon,           // 1.7.10 (id: 360)| 1.13 (id: 25172)
    PumpkinSeeds,    // 1.7.10 (id: 361)| 1.13 (id: 28985)
    MelonSeeds,      // 1.7.10 (id: 362)| 1.13 (id: 18340)
    RawBeef,         // 1.7.10 (id: 363)
    CookedBeef,      // 1.7.10 (id: 364)| 1.13 (id: 21595)
    RawChicken,      // 1.7.10 (id: 365)
    CookedChicken,   // 1.7.10 (id: 366)| 1.13 (id: 20780)| 1.16.2 (id: 16984)
    RottenFlesh,     // 1.7.10 (id: 367)| 1.13 (id: 21591)
    EnderPearl,      // 1.7.10 (id: 368, stack: 16)| 1.13 (id: 5259)
    BlazeRod,        // 1.7.10 (id: 369)| 1.13 (id: 8289)
    GhastTear,       // 1.7.10 (id: 370)| 1.13 (id: 18222)
    GoldNugget,      // 1.7.10 (id: 371)| 1.13 (id: 28814)
    NetherStalk,     // 1.7.10 (id: 372)
    Potion,          // 1.7.10(id; 373, stack: 1, durability: 0 )| 1.13 (id: 24020)
    GlassBottle,     // 1.7.10 (id: 374)| 1.13 (id: 6116)
    SpiderEye,       // 1.7.10 (id: 375)| 1.13 (id: 9318)
    FermentedSpiderEye, // 1.7.10 (id: 376)| 1.13 (id: 19386)
    BlazePowder,     // 1.7.10 (id: 377)| 1.13 (id: 18941)
    MagmaCream,      // 1.7.10 (id: 378)| 1.13 (id: 25097)
    BrewingStandItem, // 1.7.10 (id: 379)
    CauldronItem,    // 1.7.10 (id: 380)
    EyeOfEnder,      // 1.7.10 (id: 381)
    SpeckledMelon,   // 1.7.10 (id: 382)
    MonsterEgg,      // 1.7.10(id; 383, stack: 64, durability: 0 )
    ExpBottle,       // 1.7.10 (id: 384, stack: 64)
    Fireball,        // 1.7.10 (id: 385, stack: 64)
    BookAndQuill,    // 1.7.10 (id: 386, stack: 1)
    WrittenBook,     // 1.7.10 (id: 387, stack: 16)| 1.13 (id: 24164)
    Emerald,         // 1.7.10 (id: 388, stack: 64)| 1.13 (id: 5654,stack: 0)
    ItemFrame,       // 1.7.10 (id: 389)| 1.13 (id: 27318)
    FlowerPotItem,   // 1.7.10 (id: 390)
    CarrotItem,      // 1.7.10 (id: 391)
    PotatoItem,      // 1.7.10 (id: 392)
    BakedPotato,     // 1.7.10 (id: 393)| 1.13 (id: 14624)
    PoisonousPotato, // 1.7.10 (id: 394)| 1.13 (id: 32640)
    EmptyMap,        // 1.7.10 (id: 395)
    GoldenCarrot,    // 1.7.10 (id: 396)| 1.13 (id: 5300)
    SkullItem,       // 1.7.10 (id: 397)
    CarrotStick,     // 1.7.10(id; 398, stack: 1, durability: 25 )
    NetherStar,      // 1.7.10 (id: 399)| 1.13 (id: 12469)
    PumpkinPie,      // 1.7.10 (id: 400)| 1.13 (id: 28725)
    Firework,        // 1.7.10 (id: 401)
    FireworkCharge,  // 1.7.10 (id: 402)
    EnchantedBook,   // 1.7.10 (id: 403, stack: 1)| 1.13 (id: 11741)
    RedstoneComparator, // 1.7.10 (id: 404)
    NetherBrickItem, // 1.7.10 (id: 405)
    Quartz,          // 1.7.10 (id: 406)| 1.13 (id: 23608)
    ExplosiveMinecart, // 1.7.10 (id: 407, stack: 1)
    HopperMinecart,  // 1.7.10 (id: 408, stack: 1)| 1.13 (id: 19024)
    IronBarding,     // 1.7.10 (id: 417, stack: 1)
    GoldBarding,     // 1.7.10 (id: 418, stack: 1)
    DiamondBarding,  // 1.7.10 (id: 419, stack: 1)
    Leash,           // 1.7.10 (id: 420)
    NameTag,         // 1.7.10 (id: 421)| 1.13 (id: 30731)
    CommandMinecart, // 1.7.10 (id: 422, stack: 1)
    GoldRecord,      // 1.7.10 (id: 2256, stack: 1)
    GreenRecord,     // 1.7.10 (id: 2257, stack: 1)
    Record3,         // 1.7.10 (id: 2258, stack: 1)
    Record4,         // 1.7.10 (id: 2259, stack: 1)
    Record5,         // 1.7.10 (id: 2260, stack: 1)
    Record6,         // 1.7.10 (id: 2261, stack: 1)
    Record7,         // 1.7.10 (id: 2262, stack: 1)
    Record8,         // 1.7.10 (id: 2263, stack: 1)
    Record9,         // 1.7.10 (id: 2264, stack: 1)
    Record10,        // 1.7.10 (id: 2265, stack: 1)
    Record11,        // 1.7.10 (id: 2266, stack: 1)
    Record12,        // 1.7.10 (id: 2267, stack: 1)
    SlimeBlock,      // 1.8 (id: 165)| 1.13 (id: 31892)
    Barrier,         // 1.8 (id: 166)| 1.13 (id: 26453)
    IronTrapdoor,    // 1.8 (id: 167, stack: 0)| 1.13 (id: 17095)
    Prismarine,      // 1.8 (id: 168)| 1.13 (id: 7539)
    SeaLantern,      // 1.8 (id: 169)| 1.13 (id: 16984)| 1.16.2 (id: 20780)
    StandingBanner,  // 1.8 (id: 176, stack: 0)
    WallBanner,      // 1.8 (id: 177, stack: 0)
    DaylightDetectorInverted, // 1.8 (id: 178)
    RedSandstone,    // 1.8 (id: 179)| 1.13 (id: 9092)
    RedSandstoneStairs, // 1.8 (id: 180, stack: 0)| 1.13 (id: 25466)
    DoubleStoneSlab2, // 1.8 (id: 181)
    StoneSlab2,      // 1.8 (id: 182)
    SpruceFenceGate, // 1.8 (id: 183, stack: 0)| 1.13 (id: 26423)
    BirchFenceGate,  // 1.8 (id: 184, stack: 0)| 1.13 (id: 6322)
    JungleFenceGate, // 1.8 (id: 185, stack: 0)| 1.13 (id: 21360)
    DarkOakFenceGate, // 1.8 (id: 186, stack: 0)| 1.13 (id: 10679)
    AcaciaFenceGate, // 1.8 (id: 187, stack: 0)| 1.13 (id: 14145)
    SpruceFence,     // 1.8 (id: 188)| 1.13 (id: 25416)
    BirchFence,      // 1.8 (id: 189)| 1.13 (id: 17347)
    JungleFence,     // 1.8 (id: 190)| 1.13 (id: 14358)
    DarkOakFence,    // 1.8 (id: 191)| 1.13 (id: 21767)
    AcaciaFence,     // 1.8 (id: 192)| 1.13 (id: 4569)
    SpruceDoor,      // 1.8 (id: 193)| 1.13 (id: 10642)
    BirchDoor,       // 1.8 (id: 194)| 1.13 (id: 14759)
    JungleDoor,      // 1.8 (id: 195)| 1.13 (id: 28163)
    AcaciaDoor,      // 1.8 (id: 196)| 1.13 (id: 23797)
    DarkOakDoor,     // 1.8 (id: 197)| 1.13 (id: 10669)
    PrismarineShard, // 1.8 (id: 409)| 1.13 (id: 10993)
    PrismarineCrystals, // 1.8 (id: 410)| 1.13 (id: 31546)
    Rabbit,          // 1.8 (id: 411)| 1.13 (id: 23068)
    CookedRabbit,    // 1.8 (id: 412)| 1.13 (id: 4454)
    RabbitStew,      // 1.8 (id: 413, stack: 1)| 1.13 (id: 10611)
    RabbitFoot,      // 1.8 (id: 414)| 1.13 (id: 13864)
    RabbitHide,      // 1.8 (id: 415)| 1.13 (id: 12467)
    ArmorStand,      // 1.8 (id: 416, stack: 16)| 1.13 (id: 12852)
    Mutton,          // 1.8 (id: 423)| 1.13 (id: 4792)
    CookedMutton,    // 1.8 (id: 424)| 1.13 (id: 31447)
    Banner,          // 1.8 (id: 425, stack: 16)
    SpruceDoorItem,  // 1.8 (id: 427)
    BirchDoorItem,   // 1.8 (id: 428)
    JungleDoorItem,  // 1.8 (id: 429)
    AcaciaDoorItem,  // 1.8 (id: 430)
    DarkOakDoorItem, // 1.8 (id: 431)
    EndRod,          // 1.9 (id: 198)| 1.13 (id: 24832)
    ChorusPlant,     // 1.9 (id: 199)| 1.13 (id: 28243)
    ChorusFlower,    // 1.9 (id: 200)| 1.13 (id: 28542)
    PurpurBlock,     // 1.9 (id: 201)| 1.13 (id: 7538)
    PurpurPillar,    // 1.9 (id: 202)| 1.13 (id: 26718)
    PurpurStairs,    // 1.9 (id: 203, stack: 0)| 1.13 (id: 8921)
    PurpurDoubleSlab, // 1.9 (id: 204)
    PurpurSlab,      // 1.9 (id: 205)| 1.13 (id: 11487)
    EndBricks,       // 1.9 (id: 206)
    BeetrootBlock,   // 1.9 (id: 207, stack: 0)
    GrassPath,       // 1.9 (id: 208)| 1.13 (id: 8604)
    EndGateway,      // 1.9 (id: 209)| 1.13 (id: 26605)
    CommandRepeating, // 1.9 (id: 210)
    CommandChain,    // 1.9 (id: 211)
    StructureBlock,  // 1.9 (id: 255)| 1.13 (id: 26831)
    EndCrystal,      // 1.9 (id: 426)| 1.13 (id: 19090)
    ChorusFruit,     // 1.9 (id: 432)| 1.13 (id: 7652)
    ChorusFruitPopped, // 1.9 (id: 433)
    Beetroot,        // 1.9 (id: 434)| 1.13 (id: 23305)
    BeetrootSeeds,   // 1.9 (id: 435)| 1.13 (id: 21282)
    BeetrootSoup,    // 1.9 (id: 436, stack: 1)| 1.13 (id: 16036)
    DragonsBreath,   // 1.9 (id: 437)
    SplashPotion,    // 1.9 (id: 438, stack: 1)| 1.13 (id: 30248)
    SpectralArrow,   // 1.9 (id: 439)| 1.13 (id: 4568)
    TippedArrow,     // 1.9 (id: 440)| 1.13 (id: 25164)
    LingeringPotion, // 1.9(id; 441, stack: 1, durability: 0 )| 1.13 (id: 25857)
    Shield, // 1.9 (id: 442, stack: 1)| 1.9.2 (durability: 336)| 1.13 (id: 29943)durability: 336)
    Elytra, // 1.9 (id: 443, stack: 1)| 1.9.2 (durability: 431)| 1.13 (id: 23829)durability: 432)
    BoatSpruce, // 1.9 (id: 444, stack: 1)
    BoatBirch, // 1.9 (id: 445, stack: 1)
    BoatJungle, // 1.9 (id: 446, stack: 1)
    BoatAcacia, // 1.9 (id: 447, stack: 1)
    BoatDarkOak, // 1.9 (id: 448, stack: 1)
    FrostedIce, // 1.9.2 (id: 212)| 1.13 (id: 21814)
    Magma,  // 1.10 (id: 213)
    NetherWartBlock, // 1.10 (id: 214)| 1.13 (id: 15486)
    RedNetherBrick, // 1.10 (id: 215)
    BoneBlock, // 1.10 (id: 216)| 1.13 (id: 17312)
    StructureVoid, // 1.10 (id: 217)| 1.13 (id: 30806)
    Observer, // 1.11 (id: 218)| 1.13 (id: 10726)
    WhiteShulkerBox, // 1.11 (id: 219, stack: 1)| 1.13 (id: 31750)
    OrangeShulkerBox, // 1.11 (id: 220, stack: 1)| 1.13 (id: 21673)
    MagentaShulkerBox, // 1.11 (id: 221, stack: 1)| 1.13 (id: 21566)
    LightBlueShulkerBox, // 1.11 (id: 222, stack: 1)| 1.13 (id: 18226)
    YellowShulkerBox, // 1.11 (id: 223, stack: 1)| 1.13 (id: 28700)
    LimeShulkerBox, // 1.11 (id: 224, stack: 1)| 1.13 (id: 28360)
    PinkShulkerBox, // 1.11 (id: 225, stack: 1)| 1.13 (id: 24968)
    GrayShulkerBox, // 1.11 (id: 226, stack: 1)| 1.13 (id: 12754)
    SilverShulkerBox, // 1.11 (id: 227, stack: 1)
    CyanShulkerBox, // 1.11 (id: 228, stack: 1)| 1.13 (id: 28123)
    PurpleShulkerBox, // 1.11 (id: 229, stack: 1)| 1.13 (id: 10373)
    BlueShulkerBox, // 1.11 (id: 230, stack: 1)| 1.13 (id: 11476)
    BrownShulkerBox, // 1.11 (id: 231, stack: 1)| 1.13 (id: 24230)
    GreenShulkerBox, // 1.11 (id: 232, stack: 1)| 1.13 (id: 9377)
    RedShulkerBox, // 1.11 (id: 233, stack: 1)| 1.13 (id: 32448)
    BlackShulkerBox, // 1.11 (id: 234, stack: 1)| 1.13 (id: 24076)
    Totem,  // 1.11 (id: 449, stack: 1)
    ShulkerShell, // 1.11 (id: 450)| 1.13 (id: 27848)
    IronNugget, // 1.11.1 (id: 452)| 1.13 (id: 13715)
    WhiteGlazedTerracotta, // 1.12 (id: 235)| 1.13 (id: 11326)
    OrangeGlazedTerracotta, // 1.12 (id: 236)| 1.13 (id: 27451)
    MagentaGlazedTerracotta, // 1.12 (id: 237)| 1.13 (id: 8067)
    LightBlueGlazedTerracotta, // 1.12 (id: 238)| 1.13 (id: 4336)
    YellowGlazedTerracotta, // 1.12 (id: 239)| 1.13 (id: 10914)
    LimeGlazedTerracotta, // 1.12 (id: 240)| 1.13 (id: 13861)
    PinkGlazedTerracotta, // 1.12 (id: 241)| 1.13 (id: 10260)
    GrayGlazedTerracotta, // 1.12 (id: 242)| 1.13 (id: 6256)
    SilverGlazedTerracotta, // 1.12 (id: 243)
    CyanGlazedTerracotta, // 1.12 (id: 244)| 1.13 (id: 9550)
    PurpleGlazedTerracotta, // 1.12 (id: 245)| 1.13 (id: 4818)
    BlueGlazedTerracotta, // 1.12 (id: 246)| 1.13 (id: 23823)
    BrownGlazedTerracotta, // 1.12 (id: 247)| 1.13 (id: 5655)
    GreenGlazedTerracotta, // 1.12 (id: 248)| 1.13 (id: 6958)
    RedGlazedTerracotta, // 1.12 (id: 249)| 1.13 (id: 24989)
    BlackGlazedTerracotta, // 1.12 (id: 250)| 1.13 (id: 29678)
    Concrete, // 1.12 (id: 251)
    ConcretePowder, // 1.12 (id: 252)
    KnowledgeBook, // 1.12 (id: 453, stack: 1)| 1.13 (id: 12646)
    AcaciaBoat, // 1.13 (id: 27326, stack: 1)
    AcaciaButton, // 1.13 (id: 13993, stack: 0)
    AcaciaLeaves, // 1.13 (id: 16606, stack: 0)
    AcaciaLog, // 1.13 (id: 8385, stack: 0)
    AcaciaPlanks, // 1.13 (id: 31312)
    AcaciaPressurePlate, // 1.13 (id: 17586, stack: 0)
    AcaciaSapling, // 1.13 (id: 20806, stack: 0)
    AcaciaSlab, // 1.13 (id: 23730, stack: 0)
    AcaciaTrapdoor, // 1.13 (id: 18343, stack: 0)
    AcaciaWood, // 1.13 (id: 9541, stack: 0)
    Allium, // 1.13 (id: 6871)
    Andesite, // 1.13 (id: 25975)
    AttachedMelonStem, // 1.13 (id: 30882, stack: 0)
    AttachedPumpkinStem, // 1.13 (id: 12724, stack: 0)
    AzureBluet, // 1.13 (id: 17608)
    BatSpawnEgg, // 1.13 (id: 14607)
    Beef,   // 1.13 (id: 4803)
    Beetroots, // 1.13 (id: 22075, stack: 0)
    BirchBoat, // 1.13 (id: 28104, stack: 1)
    BirchButton, // 1.13 (id: 26934, stack: 0)
    BirchLeaves, // 1.13 (id: 12601, stack: 0)
    BirchLog, // 1.13 (id: 26727, stack: 0)
    BirchPlanks, // 1.13 (id: 29322)
    BirchPressurePlate, // 1.13 (id: 9664, stack: 0)
    BirchSapling, // 1.13 (id: 31533, stack: 0)
    BirchSlab, // 1.13 (id: 13807, stack: 0)
    BirchStairs, // 1.13 (id: 7657, stack: 0)
    BirchTrapdoor, // 1.13 (id: 32585, stack: 0)
    BirchWood, // 1.13 (id: 20913, stack: 0)
    BlackBanner, // 1.13(id; 9365, stack: 16, durability: 0 )
    BlackBed, // 1.13(id; 20490, stack: 1, durability: 0 )
    BlackCarpet, // 1.13 (id: 6056)
    BlackConcrete, // 1.13 (id: 13338)
    BlackConcretePowder, // 1.13 (id: 16150)
    BlackStainedGlass, // 1.13 (id: 13941)
    BlackStainedGlassPane, // 1.13 (id: 13201, stack: 0)
    BlackTerracotta, // 1.13 (id: 26691)
    BlackWallBanner, // 1.13 (id: 4919, stack: 0)
    BlackWool, // 1.13 (id: 16693)
    BlazeSpawnEgg, // 1.13 (id: 4759)
    BlueBanner, // 1.13(id; 18481, stack: 16, durability: 0 )
    BlueBed, // 1.13(id; 12714, stack: 1, durability: 0 )
    BlueCarpet, // 1.13 (id: 13292)
    BlueConcrete, // 1.13 (id: 18756)
    BlueConcretePowder, // 1.13 (id: 17773)
    BlueIce, // 1.13 (id: 22449)
    BlueOrchid, // 1.13 (id: 13432)
    BlueStainedGlass, // 1.13 (id: 7107)
    BlueStainedGlassPane, // 1.13 (id: 28484, stack: 0)
    BlueTerracotta, // 1.13 (id: 5236)
    BlueWallBanner, // 1.13 (id: 17757, stack: 0)
    BlueWool, // 1.13 (id: 15738)
    BoneMeal, // 1.13 (id: 32458)
    BrainCoral, // 1.13 (id: 31316)
    BrainCoralBlock, // 1.13 (id: 30618)
    BrainCoralFan, // 1.13 (id: 13849, stack: 0)
    BrainCoralWallFan, // 1.13 (id: 22685, stack: 0)
    Bricks, // 1.13 (id: 14165)
    BrickSlab, // 1.13 (id: 26333, stack: 0)
    BrownBanner, // 1.13(id; 11481, stack: 16, durability: 0 )
    BrownBed, // 1.13(id; 25624, stack: 1, durability: 0 )| 1.16.2 (id: 26672)
    BrownCarpet, // 1.13 (id: 23352)
    BrownConcrete, // 1.13 (id: 19006)
    BrownConcretePowder, // 1.13 (id: 21485)
    BrownMushroomBlock, // 1.13 (id: 6291, stack: 0)
    BrownStainedGlass, // 1.13 (id: 20945)
    BrownStainedGlassPane, // 1.13 (id: 17557, stack: 0)
    BrownTerracotta, // 1.13 (id: 23664)
    BrownWallBanner, // 1.13 (id: 14731, stack: 0)
    BrownWool, // 1.13 (id: 32638)
    BubbleColumn, // 1.13 (id: 13758, stack: 0)| 1.16.2 (id: 31612)
    BubbleCoral, // 1.13 (id: 12464)
    BubbleCoralBlock, // 1.13 (id: 15437)
    BubbleCoralFan, // 1.13 (id: 10795, stack: 0)
    BubbleCoralWallFan, // 1.13 (id: 20382, stack: 0)
    CactusGreen, // 1.13 (id: 17296)
    Carrots, // 1.13 (id: 17258, stack: 0)
    CarrotOnAStick, // 1.13(id; 27809, stack: 1, durability: 25 )
    CarvedPumpkin, // 1.13 (id: 25833, stack: 0)
    CaveAir, // 1.13 (id: 17422)
    CaveSpiderSpawnEgg, // 1.13 (id: 23341)
    ChainCommandBlock, // 1.13 (id: 26798, stack: 0)
    Charcoal, // 1.13 (id: 5390)
    ChestMinecart, // 1.13 (id: 4497, stack: 1)
    Chicken, // 1.13 (id: 17281)
    ChickenSpawnEgg, // 1.13 (id: 5462)
    ChippedAnvil, // 1.13 (id: 10623, stack: 0)
    ChiseledQuartzBlock, // 1.13 (id: 30964)
    ChiseledRedSandstone, // 1.13 (id: 15529)
    ChiseledSandstone, // 1.13 (id: 31763)
    ChiseledStoneBricks, // 1.13 (id: 9087)
    Clock,  // 1.13 (id: 14980)
    CoarseDirt, // 1.13 (id: 15411)
    CobblestoneSlab, // 1.13 (id: 6340, stack: 0)
    CobblestoneWall, // 1.13 (id: 12616, stack: 0)
    Cobweb, // 1.13 (id: 9469)
    CocoaBeans, // 1.13 (id: 27381)| 1.16.2 (id: 30186)
    Cod,    // 1.13 (id: 24691)
    CodBucket, // 1.13 (id: 28601, stack: 1)
    CodSpawnEgg, // 1.13 (id: 27248)
    CommandBlock, // 1.13 (id: 4355, stack: 0)
    CommandBlockMinecart, // 1.13 (id: 7992, stack: 1)
    Comparator, // 1.13 (id: 18911, stack: 0)
    Conduit, // 1.13 (id: 5148)
    CookedCod, // 1.13 (id: 9681)
    CookedPorkchop, // 1.13 (id: 27231)
    CookedSalmon, // 1.13 (id: 5615)
    CowSpawnEgg, // 1.13 (id: 14761)
    CrackedStoneBricks, // 1.13 (id: 27869)
    CraftingTable, // 1.13 (id: 20706)
    CreeperHead, // 1.13 (id: 29146, stack: 0)
    CreeperSpawnEgg, // 1.13 (id: 9653)
    CreeperWallHead, // 1.13 (id: 30123, stack: 0)
    CutRedSandstone, // 1.13 (id: 26842)| 1.17 (id: 29108)
    CutSandstone, // 1.13 (id: 6118)
    CyanBanner, // 1.13(id; 9839, stack: 16, durability: 0 )
    CyanBed, // 1.13(id; 16746, stack: 1, durability: 0 )
    CyanCarpet, // 1.13 (id: 31495)| 1.16.2 (id: 9742)
    CyanConcrete, // 1.13 (id: 26522)
    CyanConcretePowder, // 1.13 (id: 15734)
    CyanDye, // 1.13 (id: 8043)
    CyanStainedGlass, // 1.13 (id: 30604)
    CyanStainedGlassPane, // 1.13 (id: 11784, stack: 0)
    CyanTerracotta, // 1.13 (id: 25940)
    CyanWallBanner, // 1.13 (id: 10889, stack: 0)
    CyanWool, // 1.13 (id: 12221)
    DamagedAnvil, // 1.13 (id: 10274, stack: 0)
    Dandelion, // 1.13 (id: 30558)
    DandelionYellow, // 1.13 (id: 21789)
    DarkOakBoat, // 1.13 (id: 28618, stack: 1)
    DarkOakButton, // 1.13 (id: 6214, stack: 0)
    DarkOakLeaves, // 1.13 (id: 22254, stack: 0)
    DarkOakLog, // 1.13 (id: 14831, stack: 0)
    DarkOakPlanks, // 1.13 (id: 20869)
    DarkOakPressurePlate, // 1.13 (id: 31375, stack: 0)
    DarkOakSapling, // 1.13 (id: 14933, stack: 0)
    DarkOakSlab, // 1.13 (id: 28852, stack: 0)
    DarkOakTrapdoor, // 1.13 (id: 10355, stack: 0)
    DarkOakWood, // 1.13 (id: 16995, stack: 0)
    DarkPrismarine, // 1.13 (id: 19940)
    DarkPrismarineSlab, // 1.13 (id: 7577, stack: 0)
    DarkPrismarineStairs, // 1.13 (id: 26511, stack: 0)
    DeadBrainCoralBlock, // 1.13 (id: 12979)
    DeadBrainCoralFan, // 1.13 (id: 26150, stack: 0)
    DeadBrainCoralWallFan, // 1.13 (id: 23718, stack: 0)
    DeadBubbleCoralBlock, // 1.13 (id: 28220)
    DeadBubbleCoralFan, // 1.13 (id: 17322, stack: 0)
    DeadBubbleCoralWallFan, // 1.13 (id: 18453, stack: 0)
    DeadFireCoralBlock, // 1.13 (id: 5307)
    DeadFireCoralFan, // 1.13 (id: 27073, stack: 0)
    DeadFireCoralWallFan, // 1.13 (id: 23375, stack: 0)
    DeadHornCoralBlock, // 1.13 (id: 15103)
    DeadHornCoralFan, // 1.13 (id: 11387, stack: 0)
    DeadHornCoralWallFan, // 1.13 (id: 27550, stack: 0)
    DeadTubeCoralBlock, // 1.13 (id: 28350)
    DeadTubeCoralFan, // 1.13 (id: 17628, stack: 0)
    DeadTubeCoralWallFan, // 1.13 (id: 5128, stack: 0)
    DebugStick, // 1.13 (id: 24562, stack: 1)
    DiamondHorseArmor, // 1.13 (id: 10321, stack: 1)
    DiamondShovel, // 1.13(id; 25415, stack: 1, durability: 1561 )
    Diorite, // 1.13 (id: 24688)
    DolphinSpawnEgg, // 1.13 (id: 20787)
    DonkeySpawnEgg, // 1.13 (id: 14513)
    DragonBreath, // 1.13 (id: 20154)
    DragonHead, // 1.13 (id: 20084, stack: 0)
    DragonWallHead, // 1.13 (id: 19818, stack: 0)
    DriedKelp, // 1.13 (id: 21042)
    DriedKelpBlock, // 1.13 (id: 12966)
    DrownedSpawnEgg, // 1.13 (id: 19368)
    ElderGuardianSpawnEgg, // 1.13 (id: 11418)
    EnchantedGoldenApple, // 1.13 (id: 8280)
    EnchantingTable, // 1.13 (id: 16255)
    EndermanSpawnEgg, // 1.13 (id: 29488)
    EndermiteSpawnEgg, // 1.13 (id: 16617)
    EnderEye, // 1.13 (id: 24860)
    EndPortal, // 1.13 (id: 16782)
    EndPortalFrame, // 1.13 (id: 15480, stack: 0)
    EndStone, // 1.13 (id: 29686)
    EndStoneBricks, // 1.13 (id: 20314)
    EvokerSpawnEgg, // 1.13 (id: 21271)
    ExperienceBottle, // 1.13 (id: 12858)
    Farmland, // 1.13 (id: 31166, stack: 0)
    Fern,   // 1.13 (id: 15794)
    FilledMap, // 1.13 (id: 23504)
    FireworkRocket, // 1.13 (id: 23841)
    FireworkStar, // 1.13 (id: 12190)
    FireCharge, // 1.13 (id: 4842)
    FireCoral, // 1.13 (id: 29151)
    FireCoralBlock, // 1.13 (id: 12119)
    FireCoralFan, // 1.13 (id: 11112, stack: 0)
    FireCoralWallFan, // 1.13 (id: 20100, stack: 0)
    FurnaceMinecart, // 1.13 (id: 14196, stack: 1)
    GhastSpawnEgg, // 1.13 (id: 9970)
    GlassPane, // 1.13 (id: 5709, stack: 0)
    GlisteringMelonSlice, // 1.13 (id: 20158)
    GoldenAxe, // 1.13(id; 4878, stack: 1, durability: 32 )
    GoldenBoots, // 1.13(id; 7859, stack: 1, durability: 91 )
    GoldenChestplate, // 1.13(id; 4507, stack: 1, durability: 112 )
    GoldenHelmet, // 1.13(id; 7945, stack: 1, durability: 77 )
    GoldenHoe, // 1.13(id; 19337, stack: 1, durability: 32 )
    GoldenHorseArmor, // 1.13 (id: 7996, stack: 1)
    GoldenLeggings, // 1.13(id; 21002, stack: 1, durability: 105 )
    GoldenPickaxe, // 1.13(id; 10901, stack: 1, durability: 32 )| 1.16.2 (id: 25898)
    GoldenShovel, // 1.13(id; 15597, stack: 1, durability: 32 )
    GoldenSword, // 1.13(id; 10505, stack: 1, durability: 32 )
    Granite, // 1.13 (id: 21091)
    GrassBlock, // 1.13 (id: 28346, stack: 0)
    GrayBanner, // 1.13(id; 12053, stack: 16, durability: 0 )
    GrayBed, // 1.13(id; 15745, stack: 1, durability: 0 )
    GrayCarpet, // 1.13 (id: 26991)
    GrayConcrete, // 1.13 (id: 13959)
    GrayConcretePowder, // 1.13 (id: 13031)
    GrayDye, // 1.13 (id: 9184)
    GrayStainedGlass, // 1.13 (id: 29979)
    GrayStainedGlassPane, // 1.13 (id: 25272, stack: 0)
    GrayTerracotta, // 1.13 (id: 18004)
    GrayWallBanner, // 1.13 (id: 24275, stack: 0)
    GrayWool, // 1.13 (id: 27209)
    GreenBanner, // 1.13(id; 10698, stack: 16, durability: 0 )
    GreenBed, // 1.13(id; 13797, stack: 1, durability: 0 )
    GreenCarpet, // 1.13 (id: 7780)
    GreenConcrete, // 1.13 (id: 17949)
    GreenConcretePowder, // 1.13 (id: 6904)
    GreenStainedGlass, // 1.13 (id: 22503)
    GreenStainedGlassPane, // 1.13 (id: 4767, stack: 0)
    GreenTerracotta, // 1.13 (id: 4105)
    GreenWallBanner, // 1.13 (id: 15046, stack: 0)
    GreenWool, // 1.13 (id: 25085)
    GuardianSpawnEgg, // 1.13 (id: 20113)
    Gunpowder, // 1.13 (id: 29974)
    HeartOfTheSea, // 1.13 (id: 11807)
    HeavyWeightedPressurePlate, // 1.13 (id: 16970, stack: 0)
    HornCoral, // 1.13 (id: 19511)
    HornCoralBlock, // 1.13 (id: 19958)
    HornCoralFan, // 1.13 (id: 13610, stack: 0)
    HornCoralWallFan, // 1.13 (id: 28883, stack: 0)
    HorseSpawnEgg, // 1.13 (id: 25981)
    HuskSpawnEgg, // 1.13 (id: 20178)
    InfestedChiseledStoneBricks, // 1.13 (id: 4728)
    InfestedCobblestone, // 1.13 (id: 28798)| 1.16.2 (id: 4348)
    InfestedCrackedStoneBricks, // 1.13 (id: 7476)
    InfestedMossyStoneBricks, // 1.13 (id: 9850)
    InfestedStone, // 1.13 (id: 18440)
    InfestedStoneBricks, // 1.13 (id: 19749)
    InkSac, // 1.13 (id: 7184)
    IronBars, // 1.13 (id: 9378, stack: 0)
    IronHorseArmor, // 1.13 (id: 30108, stack: 1)
    IronShovel, // 1.13(id; 30045, stack: 1, durability: 250 )
    JungleBoat, // 1.13 (id: 4495, stack: 1)
    JungleButton, // 1.13 (id: 25317, stack: 0)
    JungleLeaves, // 1.13 (id: 5133, stack: 0)
    JungleLog, // 1.13 (id: 20721, stack: 0)
    JunglePlanks, // 1.13 (id: 26445)
    JunglePressurePlate, // 1.13 (id: 11376, stack: 0)
    JungleSapling, // 1.13 (id: 17951, stack: 0)
    JungleSlab, // 1.13 (id: 19117, stack: 0)
    JungleStairs, // 1.13 (id: 20636, stack: 0)
    JungleTrapdoor, // 1.13 (id: 8626, stack: 0)
    JungleWood, // 1.13 (id: 10341, stack: 0)
    Kelp,   // 1.13 (id: 21916, stack: 0)
    KelpPlant, // 1.13 (id: 29697)
    LapisLazuli, // 1.13 (id: 11075)
    LargeFern, // 1.13 (id: 30177, stack: 0)
    Lead,   // 1.13 (id: 29539)
    LightBlueBanner, // 1.13(id; 18060, stack: 16, durability: 0 )
    LightBlueBed, // 1.13(id; 20957, stack: 1, durability: 0 )
    LightBlueCarpet, // 1.13 (id: 21194)
    LightBlueConcrete, // 1.13 (id: 29481)
    LightBlueConcretePowder, // 1.13 (id: 31206)
    LightBlueDye, // 1.13 (id: 28738)
    LightBlueStainedGlass, // 1.13 (id: 17162)
    LightBlueStainedGlassPane, // 1.13 (id: 18721, stack: 0)
    LightBlueTerracotta, // 1.13 (id: 31779)
    LightBlueWallBanner, // 1.13 (id: 12011, stack: 0)
    LightBlueWool, // 1.13 (id: 21073)
    LightGrayBanner, // 1.13(id; 11417, stack: 16, durability: 0 )
    LightGrayBed, // 1.13(id; 5090, stack: 1, durability: 0 )
    LightGrayCarpet, // 1.13 (id: 11317)
    LightGrayConcrete, // 1.13 (id: 14453)
    LightGrayConcretePowder, // 1.13 (id: 21589)
    LightGrayDye, // 1.13 (id: 27643)
    LightGrayGlazedTerracotta, // 1.13 (id: 10707, stack: 0)
    LightGrayShulkerBox, // 1.13(id; 21345, stack: 1, durability: 0 )
    LightGrayStainedGlass, // 1.13 (id: 5843)
    LightGrayStainedGlassPane, // 1.13 (id: 19008, stack: 0)
    LightGrayTerracotta, // 1.13 (id: 26388)
    LightGrayWallBanner, // 1.13 (id: 31088, stack: 0)
    LightGrayWool, // 1.13 (id: 22936)
    LightWeightedPressurePlate, // 1.13 (id: 14875, stack: 0)
    Lilac,  // 1.13 (id: 22837, stack: 0)
    LilyPad, // 1.13 (id: 19271)
    LimeBanner, // 1.13(id; 18887, stack: 16, durability: 0 )
    LimeBed, // 1.13(id; 27860, stack: 1, durability: 0 )
    LimeCarpet, // 1.13 (id: 15443)
    LimeConcrete, // 1.13 (id: 5863)
    LimeConcretePowder, // 1.13 (id: 28859)
    LimeDye, // 1.13 (id: 6147)
    LimeStainedGlass, // 1.13 (id: 24266)
    LimeStainedGlassPane, // 1.13 (id: 10610, stack: 0)
    LimeTerracotta, // 1.13 (id: 24013)
    LimeWallBanner, // 1.13 (id: 21422, stack: 0)
    LimeWool, // 1.13 (id: 10443)
    LlamaSpawnEgg, // 1.13 (id: 23640)
    MagentaBanner, // 1.13(id; 15591, stack: 16, durability: 0 )
    MagentaBed, // 1.13(id; 20061, stack: 1, durability: 0 )
    MagentaCarpet, // 1.13 (id: 6180)
    MagentaConcrete, // 1.13 (id: 20591)
    MagentaConcretePowder, // 1.13 (id: 8272)
    MagentaDye, // 1.13 (id: 11788)
    MagentaStainedGlass, // 1.13 (id: 26814)
    MagentaStainedGlassPane, // 1.13 (id: 14082, stack: 0)
    MagentaTerracotta, // 1.13 (id: 25900)
    MagentaWallBanner, // 1.13 (id: 23291, stack: 0)
    MagentaWool, // 1.13 (id: 11853)
    MagmaBlock, // 1.13 (id: 25927)
    MagmaCubeSpawnEgg, // 1.13 (id: 26638)
    MelonSlice, // 1.13 (id: 5347)
    MooshroomSpawnEgg, // 1.13 (id: 22125)
    MossyCobblestoneWall, // 1.13 (id: 11536, stack: 0)
    MossyStoneBricks, // 1.13 (id: 16415)
    MovingPiston, // 1.13 (id: 13831, stack: 0)
    MuleSpawnEgg, // 1.13 (id: 11229)
    MushroomStem, // 1.13 (id: 16543, stack: 0)
    MushroomStew, // 1.13 (id: 16336, stack: 1)
    MusicDisc11, // 1.13 (id: 27426, stack: 1)
    MusicDisc13, // 1.13 (id: 16359, stack: 1)
    MusicDiscBlocks, // 1.13 (id: 26667, stack: 1)
    MusicDiscCat, // 1.13 (id: 16246, stack: 1)
    MusicDiscChirp, // 1.13 (id: 19436, stack: 1)
    MusicDiscFar, // 1.13 (id: 13823, stack: 1)| 1.17 (id: 31742)
    MusicDiscMall, // 1.13 (id: 11517, stack: 1)
    MusicDiscMellohi, // 1.13 (id: 26117, stack: 1)
    MusicDiscStal, // 1.13 (id: 14989, stack: 1)
    MusicDiscStrad, // 1.13 (id: 16785, stack: 1)
    MusicDiscWait, // 1.13 (id: 26499, stack: 1)
    MusicDiscWard, // 1.13 (id: 24026, stack: 1)
    Mycelium, // 1.13 (id: 9913, stack: 0)
    NautilusShell, // 1.13 (id: 19989)
    NetherBricks, // 1.13 (id: 27802)
    NetherBrickFence, // 1.13 (id: 5286, stack: 0)
    NetherBrickSlab, // 1.13 (id: 26586, stack: 0)
    NetherPortal, // 1.13 (id: 19469, stack: 0)
    NetherQuartzOre, // 1.13 (id: 4807)
    NetherWart, // 1.13 (id: 29227, stack: 0)
    OakBoat, // 1.13 (id: 17570, stack: 1)
    OakButton, // 1.13 (id: 13510, stack: 0)
    OakDoor, // 1.13 (id: 20341, stack: 0)
    OakFence, // 1.13 (id: 6442, stack: 0)
    OakFenceGate, // 1.13 (id: 16689, stack: 0)
    OakLeaves, // 1.13 (id: 4385, stack: 0)
    OakLog, // 1.13 (id: 26723, stack: 0)
    OakPlanks, // 1.13 (id: 14905)
    OakPressurePlate, // 1.13 (id: 20108, stack: 0)
    OakSapling, // 1.13 (id: 9636, stack: 0)
    OakSlab, // 1.13 (id: 12002, stack: 0)
    OakStairs, // 1.13 (id: 5449, stack: 0)
    OakTrapdoor, // 1.13 (id: 16927, stack: 0)
    OakWood, // 1.13 (id: 7378, stack: 0)
    OcelotSpawnEgg, // 1.13 (id: 30080)
    OrangeBanner, // 1.13(id; 4839, stack: 16, durability: 0 )
    OrangeBed, // 1.13(id; 11194, stack: 1, durability: 0 )
    OrangeCarpet, // 1.13 (id: 24752)
    OrangeConcrete, // 1.13 (id: 19914)
    OrangeConcretePowder, // 1.13 (id: 30159)
    OrangeDye, // 1.13 (id: 13866)
    OrangeStainedGlass, // 1.13 (id: 25142)
    OrangeStainedGlassPane, // 1.13 (id: 21089, stack: 0)
    OrangeTerracotta, // 1.13 (id: 18684)
    OrangeTulip, // 1.13 (id: 26038)
    OrangeWallBanner, // 1.13 (id: 9936, stack: 0)
    OrangeWool, // 1.13 (id: 23957)
    OxeyeDaisy, // 1.13 (id: 11709)
    ParrotSpawnEgg, // 1.13 (id: 23614)
    Peony,  // 1.13 (id: 21155, stack: 0)
    PetrifiedOakSlab, // 1.13 (id: 18658, stack: 0)
    PhantomMembrane, // 1.13 (id: 18398)
    PhantomSpawnEgg, // 1.13 (id: 24648)
    PigSpawnEgg, // 1.13 (id: 22584)
    PinkBanner, // 1.13(id; 19439, stack: 16, durability: 0 )
    PinkBed, // 1.13(id; 13795, stack: 1, durability: 0 )
    PinkCarpet, // 1.13 (id: 30186)| 1.16.2 (id: 27381)
    PinkConcrete, // 1.13 (id: 5227)
    PinkConcretePowder, // 1.13 (id: 6421)
    PinkDye, // 1.13 (id: 31151)
    PinkStainedGlass, // 1.13 (id: 16164)
    PinkStainedGlassPane, // 1.13 (id: 24637, stack: 0)
    PinkTerracotta, // 1.13 (id: 23727)
    PinkTulip, // 1.13 (id: 27319)
    PinkWallBanner, // 1.13 (id: 9421, stack: 0)
    PinkWool, // 1.13 (id: 7611)
    Piston, // 1.13 (id: 21130, stack: 0)
    PistonHead, // 1.13 (id: 30226, stack: 0)
    PlayerHead, // 1.13 (id: 21174, stack: 0)
    PlayerWallHead, // 1.13 (id: 13164, stack: 0)
    Podzol, // 1.13 (id: 24068, stack: 0)
    PolarBearSpawnEgg, // 1.13 (id: 17015)
    PolishedAndesite, // 1.13 (id: 8335)
    PolishedDiorite, // 1.13 (id: 31615)
    PolishedGranite, // 1.13 (id: 5477)
    PoppedChorusFruit, // 1.13 (id: 27844)
    Poppy,  // 1.13 (id: 12851)
    Porkchop, // 1.13 (id: 30896)
    Potatoes, // 1.13 (id: 10879, stack: 0)
    PottedAcaciaSapling, // 1.13 (id: 14096)
    PottedAllium, // 1.13 (id: 13184)
    PottedAzureBluet, // 1.13 (id: 8754)
    PottedBirchSapling, // 1.13 (id: 32484)
    PottedBlueOrchid, // 1.13 (id: 6599)
    PottedBrownMushroom, // 1.13 (id: 14481)
    PottedCactus, // 1.13 (id: 8777)
    PottedDandelion, // 1.13 (id: 9727)
    PottedDarkOakSapling, // 1.13 (id: 6486)
    PottedDeadBush, // 1.13 (id: 13020)
    PottedFern, // 1.13 (id: 23315)
    PottedJungleSapling, // 1.13 (id: 7525)
    PottedOakSapling, // 1.13 (id: 11905)
    PottedOrangeTulip, // 1.13 (id: 28807)
    PottedOxeyeDaisy, // 1.13 (id: 19707)
    PottedPinkTulip, // 1.13 (id: 10089)
    PottedPoppy, // 1.13 (id: 7457)
    PottedRedMushroom, // 1.13 (id: 22881)
    PottedRedTulip, // 1.13 (id: 28594)
    PottedSpruceSapling, // 1.13 (id: 29498)
    PottedWhiteTulip, // 1.13 (id: 24330)
    PrismarineBricks, // 1.13 (id: 29118)
    PrismarineBrickSlab, // 1.13 (id: 26672, stack: 0)| 1.16.2 (id: 25624)
    PrismarineBrickStairs, // 1.13 (id: 15445, stack: 0)
    PrismarineSlab, // 1.13 (id: 31323, stack: 0)
    PrismarineStairs, // 1.13 (id: 19217, stack: 0)
    Pufferfish, // 1.13 (id: 8115)
    PufferfishBucket, // 1.13 (id: 8861, stack: 1)
    PufferfishSpawnEgg, // 1.13 (id: 24573)| 1.14 (id: 24570)
    PurpleBanner, // 1.13(id; 29027, stack: 16, durability: 0 )
    PurpleBed, // 1.13(id; 29755, stack: 1, durability: 0 )
    PurpleCarpet, // 1.13 (id: 5574)
    PurpleConcrete, // 1.13 (id: 20623)
    PurpleConcretePowder, // 1.13 (id: 26808)
    PurpleDye, // 1.13 (id: 6347)
    PurpleStainedGlass, // 1.13 (id: 21845)
    PurpleStainedGlassPane, // 1.13 (id: 10948, stack: 0)
    PurpleTerracotta, // 1.13 (id: 10387)
    PurpleWallBanner, // 1.13 (id: 14298, stack: 0)
    PurpleWool, // 1.13 (id: 11922)
    QuartzPillar, // 1.13 (id: 16452, stack: 0)
    QuartzSlab, // 1.13 (id: 4423, stack: 0)
    RabbitSpawnEgg, // 1.13 (id: 26496)
    Rail,   // 1.13 (id: 13285, stack: 0)
    RedstoneLamp, // 1.13 (id: 8217, stack: 0)
    RedstoneTorch, // 1.13 (id: 22547, stack: 0)
    RedstoneWallTorch, // 1.13 (id: 7595, stack: 0)
    RedBanner, // 1.13(id; 26961, stack: 16, durability: 0 )
    RedBed, // 1.13(id; 30910, stack: 1, durability: 0 )
    RedCarpet, // 1.13 (id: 5424)
    RedConcrete, // 1.13 (id: 8032)
    RedConcretePowder, // 1.13 (id: 13286)
    RedMushroomBlock, // 1.13 (id: 20766, stack: 0)
    RedNetherBricks, // 1.13 (id: 18056)
    RedSand, // 1.13 (id: 16279)
    RedSandstoneSlab, // 1.13 (id: 17550, stack: 0)
    RedStainedGlass, // 1.13 (id: 9717)
    RedStainedGlassPane, // 1.13 (id: 8630, stack: 0)
    RedTerracotta, // 1.13 (id: 5086)
    RedTulip, // 1.13 (id: 16781)
    RedWallBanner, // 1.13 (id: 4378, stack: 0)
    RedWool, // 1.13 (id: 11621)
    Repeater, // 1.13 (id: 28823, stack: 0)
    RepeatingCommandBlock, // 1.13 (id: 12405, stack: 0)
    RoseBush, // 1.13 (id: 6080, stack: 0)
    RoseRed, // 1.13 (id: 15694)
    Salmon, // 1.13 (id: 18516)
    SalmonBucket, // 1.13 (id: 31427, stack: 1)| 1.17 (id: 9606)
    SalmonSpawnEgg, // 1.13 (id: 18739)
    SandstoneSlab, // 1.13 (id: 29830, stack: 0)
    Scute,  // 1.13 (id: 11914)
    Seagrass, // 1.13 (id: 23942)
    SeaPickle, // 1.13 (id: 19562, stack: 0)
    SheepSpawnEgg, // 1.13 (id: 24488)
    ShulkerBox, // 1.13(id; 7776, stack: 1, durability: 0 )
    ShulkerSpawnEgg, // 1.13 (id: 31848)
    SilverfishSpawnEgg, // 1.13 (id: 14537)
    SkeletonHorseSpawnEgg, // 1.13 (id: 21356)
    SkeletonSkull, // 1.13 (id: 13270, stack: 0)
    SkeletonSpawnEgg, // 1.13 (id: 15261)
    SkeletonWallSkull, // 1.13 (id: 31650, stack: 0)
    SlimeSpawnEgg, // 1.13 (id: 6550)| 1.16.1 (id: 17196)
    SmoothQuartz, // 1.13 (id: 14415)
    SmoothRedSandstone, // 1.13 (id: 25180)
    SmoothSandstone, // 1.13 (id: 30039)
    SmoothStone, // 1.13 (id: 21910)
    Snowball, // 1.13 (id: 19487, stack: 16)
    Spawner, // 1.13 (id: 7018)
    SpiderSpawnEgg, // 1.13 (id: 14984)
    SpruceBoat, // 1.13 (id: 9606, stack: 1)| 1.17 (id: 31427)
    SpruceButton, // 1.13 (id: 23281, stack: 0)
    SpruceLeaves, // 1.13 (id: 20039, stack: 0)
    SpruceLog, // 1.13 (id: 9726, stack: 0)
    SprucePlanks, // 1.13 (id: 14593)
    SprucePressurePlate, // 1.13 (id: 15932, stack: 0)
    SpruceSapling, // 1.13 (id: 19874, stack: 0)
    SpruceSlab, // 1.13 (id: 4348, stack: 0)| 1.16.2 (id: 28798)
    SpruceStairs, // 1.13 (id: 11192, stack: 0)
    SpruceTrapdoor, // 1.13 (id: 10289, stack: 0)
    SpruceWood, // 1.13 (id: 32328, stack: 0)
    SquidSpawnEgg, // 1.13 (id: 10682)
    StickyPiston, // 1.13 (id: 18127, stack: 0)
    StoneBricks, // 1.13 (id: 6962)
    StoneBrickSlab, // 1.13 (id: 19676, stack: 0)
    StoneBrickStairs, // 1.13 (id: 27032, stack: 0)
    StonePressurePlate, // 1.13 (id: 22591, stack: 0)
    StoneShovel, // 1.13(id; 9520, stack: 1, durability: 131 )
    StoneSlab, // 1.13 (id: 19838, stack: 0)
    StraySpawnEgg, // 1.13 (id: 30153)
    StrippedAcaciaLog, // 1.13 (id: 18167, stack: 0)
    StrippedAcaciaWood, // 1.13 (id: 27193, stack: 0)
    StrippedBirchLog, // 1.13 (id: 8838, stack: 0)
    StrippedBirchWood, // 1.13 (id: 22350, stack: 0)
    StrippedDarkOakLog, // 1.13 (id: 6492, stack: 0)
    StrippedDarkOakWood, // 1.13 (id: 16000, stack: 0)
    StrippedJungleLog, // 1.13 (id: 15476, stack: 0)
    StrippedJungleWood, // 1.13 (id: 30315, stack: 0)
    StrippedOakLog, // 1.13 (id: 20523, stack: 0)
    StrippedOakWood, // 1.13 (id: 31455, stack: 0)
    StrippedSpruceLog, // 1.13 (id: 6140, stack: 0)
    StrippedSpruceWood, // 1.13 (id: 6467, stack: 0)
    Sunflower, // 1.13 (id: 7408, stack: 0)
    TallGrass, // 1.13 (id: 21559, stack: 0)
    TallSeagrass, // 1.13 (id: 27189, stack: 0)
    Terracotta, // 1.13 (id: 16544)
    TntMinecart, // 1.13 (id: 4277, stack: 1)
    TotemOfUndying, // 1.13 (id: 10139, stack: 1)
    Trident, // 1.13(id; 7534, stack: 1, durability: 250 )
    TropicalFish, // 1.13 (id: 24879)
    TropicalFishBucket, // 1.13 (id: 29995, stack: 1)
    TropicalFishSpawnEgg, // 1.13 (id: 19713)
    TubeCoral, // 1.13 (id: 23048)
    TubeCoralBlock, // 1.13 (id: 23723)
    TubeCoralFan, // 1.13 (id: 19929, stack: 0)
    TubeCoralWallFan, // 1.13 (id: 25282, stack: 0)
    TurtleEgg, // 1.13 (id: 32101, stack: 0)
    TurtleHelmet, // 1.13(id; 30120, stack: 1, durability: 275 )
    TurtleSpawnEgg, // 1.13 (id: 17324)
    VexSpawnEgg, // 1.13 (id: 27751)
    VillagerSpawnEgg, // 1.13 (id: 30348)
    VindicatorSpawnEgg, // 1.13 (id: 25324)
    VoidAir, // 1.13 (id: 13668)
    WallTorch, // 1.13 (id: 25890, stack: 0)
    WetSponge, // 1.13 (id: 9043)
    WheatSeeds, // 1.13 (id: 28742)
    WhiteBanner, // 1.13(id; 17562, stack: 16, durability: 0 )
    WhiteBed, // 1.13(id; 8185, stack: 1, durability: 0 )
    WhiteCarpet, // 1.13 (id: 15117)
    WhiteConcrete, // 1.13 (id: 6281)
    WhiteConcretePowder, // 1.13 (id: 10363)
    WhiteStainedGlass, // 1.13 (id: 31190)
    WhiteStainedGlassPane, // 1.13 (id: 10557, stack: 0)
    WhiteTerracotta, // 1.13 (id: 20975)
    WhiteTulip, // 1.13 (id: 9742)| 1.16.2 (id: 31495)
    WhiteWallBanner, // 1.13 (id: 15967, stack: 0)
    WhiteWool, // 1.13 (id: 8624)
    WitchSpawnEgg, // 1.13 (id: 11837)
    WitherSkeletonSkull, // 1.13 (id: 31487, stack: 0)
    WitherSkeletonSpawnEgg, // 1.13 (id: 10073)
    WitherSkeletonWallSkull, // 1.13 (id: 9326, stack: 0)
    WolfSpawnEgg, // 1.13 (id: 21692)
    WoodenAxe, // 1.13(id; 6292, stack: 1, durability: 59 )
    WoodenHoe, // 1.13(id; 16043, stack: 1, durability: 59 )
    WoodenPickaxe, // 1.13(id; 12792, stack: 1, durability: 59 )
    WoodenShovel, // 1.13(id; 28432, stack: 1, durability: 59 )
    WoodenSword, // 1.13(id; 7175, stack: 1, durability: 59 )
    WritableBook, // 1.13 (id: 13393, stack: 1)
    YellowBanner, // 1.13(id; 30382, stack: 16, durability: 0 )
    YellowBed, // 1.13(id; 30410, stack: 1, durability: 0 )
    YellowCarpet, // 1.13 (id: 18149)
    YellowConcrete, // 1.13 (id: 15722)
    YellowConcretePowder, // 1.13 (id: 10655)
    YellowStainedGlass, // 1.13 (id: 12182)
    YellowStainedGlassPane, // 1.13 (id: 20298, stack: 0)
    YellowTerracotta, // 1.13 (id: 32129)
    YellowWallBanner, // 1.13 (id: 32004, stack: 0)
    YellowWool, // 1.13 (id: 29507)
    ZombieHead, // 1.13 (id: 9304, stack: 0)
    ZombieHorseSpawnEgg, // 1.13 (id: 4275)
    ZombiePigmanSpawnEgg, // 1.13 (id: 11531)
    ZombieSpawnEgg, // 1.13 (id: 5814)
    ZombieVillagerSpawnEgg, // 1.13 (id: 10311)
    ZombieWallHead, // 1.13 (id: 16296, stack: 0)
    DeadBrainCoral, // 1.13.1 (id: 9116, stack: 0)
    DeadBubbleCoral, // 1.13.1 (id: 30583, stack: 0)
    DeadFireCoral, // 1.13.1 (id: 8365, stack: 0)
    DeadHornCoral, // 1.13.1 (id: 5755, stack: 0)
    DeadTubeCoral, // 1.13.1 (id: 18028, stack: 0)
    AcaciaSign, // 1.14(id; 29808, stack: 16, durability: 0 )
    AcaciaWallSign, // 1.14(id; 20316, stack: 16, durability: 0 )
    AndesiteSlab, // 1.14 (id: 32124, stack: 0)
    AndesiteStairs, // 1.14 (id: 17747, stack: 0)
    AndesiteWall, // 1.14 (id: 14938, stack: 0)
    Bamboo, // 1.14 (id: 18728, stack: 0)
    BambooSapling, // 1.14 (id: 8478)
    Barrel, // 1.14 (id: 22396, stack: 0)
    Bell,   // 1.14 (id: 20000, stack: 0)
    BirchSign, // 1.14(id; 11351, stack: 16, durability: 0 )
    BirchWallSign, // 1.14(id; 9887, stack: 16, durability: 0 )
    BlackDye, // 1.14 (id: 6202)
    BlastFurnace, // 1.14 (id: 31157, stack: 0)
    BlueDye, // 1.14 (id: 11588)
    BrickWall, // 1.14 (id: 18995, stack: 0)
    BrownDye, // 1.14 (id: 7648)
    Campfire, // 1.14 (id: 8488, stack: 0)
    CartographyTable, // 1.14 (id: 28529)
    CatSpawnEgg, // 1.14 (id: 29583)
    Composter, // 1.14 (id: 31247, stack: 0)
    Cornflower, // 1.14 (id: 15405)
    CreeperBannerPattern, // 1.14 (id: 15774, stack: 1)
    Crossbow, // 1.14(id; 4340, stack: 1, durability: 326 )
    CutRedSandstoneSlab, // 1.14 (id: 7220, stack: 0)
    CutSandstoneSlab, // 1.14 (id: 30944, stack: 0)
    DarkOakSign, // 1.14(id; 15127, stack: 16, durability: 0 )
    DarkOakWallSign, // 1.14(id; 9508, stack: 16, durability: 0 )
    DioriteSlab, // 1.14 (id: 10715, stack: 0)
    DioriteStairs, // 1.14 (id: 13134, stack: 0)
    DioriteWall, // 1.14 (id: 17412, stack: 0)
    EndStoneBrickSlab, // 1.14 (id: 23239, stack: 0)
    EndStoneBrickStairs, // 1.14 (id: 28831, stack: 0)
    EndStoneBrickWall, // 1.14 (id: 27225, stack: 0)
    FletchingTable, // 1.14 (id: 30838)
    FlowerBannerPattern, // 1.14 (id: 5762, stack: 1)
    FoxSpawnEgg, // 1.14 (id: 22376)
    GlobeBannerPattern, // 1.14 (id: 27753, stack: 1)
    GraniteSlab, // 1.14 (id: 25898, stack: 0)| 1.16.2 (id: 10901)
    GraniteStairs, // 1.14 (id: 21840, stack: 0)
    GraniteWall, // 1.14 (id: 23279, stack: 0)
    GreenDye, // 1.14 (id: 23215)
    Grindstone, // 1.14 (id: 26260, stack: 0)
    Jigsaw, // 1.14 (id: 17398, stack: 0)
    JungleSign, // 1.14(id; 24717, stack: 16, durability: 0 )
    JungleWallSign, // 1.14(id; 29629, stack: 16, durability: 0 )
    Lantern, // 1.14 (id: 5992, stack: 0)
    LeatherHorseArmor, // 1.14 (id: 30667, stack: 1)
    Lectern, // 1.14 (id: 23490, stack: 0)
    LilyOfTheValley, // 1.14 (id: 7185)
    Loom,   // 1.14 (id: 14276, stack: 0)
    MojangBannerPattern, // 1.14 (id: 11903, stack: 1)
    MossyCobblestoneSlab, // 1.14 (id: 12139, stack: 0)
    MossyCobblestoneStairs, // 1.14 (id: 29210, stack: 0)
    MossyStoneBrickSlab, // 1.14 (id: 14002, stack: 0)
    MossyStoneBrickStairs, // 1.14 (id: 27578, stack: 0)
    MossyStoneBrickWall, // 1.14 (id: 18259, stack: 0)
    NetherBrickWall, // 1.14 (id: 10398, stack: 0)
    OakSign, // 1.14(id; 8192, stack: 16, durability: 0 )
    OakWallSign, // 1.14(id; 12984, stack: 16, durability: 0 )
    PandaSpawnEgg, // 1.14 (id: 23759)
    PillagerSpawnEgg, // 1.14 (id: 28659)
    PolishedAndesiteSlab, // 1.14 (id: 24573, stack: 0)
    PolishedAndesiteStairs, // 1.14 (id: 7573, stack: 0)| 1.16.1 (id: 19242)
    PolishedDioriteSlab, // 1.14 (id: 18303, stack: 0)
    PolishedDioriteStairs, // 1.14 (id: 4625, stack: 0)
    PolishedGraniteSlab, // 1.14 (id: 4521, stack: 0)
    PolishedGraniteStairs, // 1.14 (id: 29588, stack: 0)
    PottedBamboo, // 1.14 (id: 22542)
    PottedCornflower, // 1.14 (id: 28917)
    PottedLilyOfTheValley, // 1.14 (id: 9364)
    PottedWitherRose, // 1.14 (id: 26876)
    PrismarineWall, // 1.14 (id: 18184, stack: 0)
    RavagerSpawnEgg, // 1.14 (id: 8726)
    RedDye, // 1.14 (id: 5728)
    RedNetherBrickSlab, // 1.14 (id: 12462, stack: 0)
    RedNetherBrickStairs, // 1.14 (id: 26374, stack: 0)
    RedNetherBrickWall, // 1.14 (id: 4580, stack: 0)
    RedSandstoneWall, // 1.14 (id: 4753, stack: 0)
    SandstoneWall, // 1.14 (id: 18470, stack: 0)
    Scaffolding, // 1.14 (id: 15757, stack: 0)
    SkullBannerPattern, // 1.14 (id: 7680, stack: 1)
    SmithingTable, // 1.14 (id: 9082)
    Smoker, // 1.14 (id: 24781, stack: 0)
    SmoothQuartzSlab, // 1.14 (id: 26543, stack: 0)
    SmoothQuartzStairs, // 1.14 (id: 19560, stack: 0)
    SmoothRedSandstoneSlab, // 1.14 (id: 16304, stack: 0)
    SmoothRedSandstoneStairs, // 1.14 (id: 17561, stack: 0)
    SmoothSandstoneSlab, // 1.14 (id: 9030, stack: 0)
    SmoothSandstoneStairs, // 1.14 (id: 21183, stack: 0)
    SmoothStoneSlab, // 1.14 (id: 24129, stack: 0)
    SpruceSign, // 1.14(id; 21502, stack: 16, durability: 0 )
    SpruceWallSign, // 1.14(id; 7352, stack: 16, durability: 0 )
    Stonecutter, // 1.14 (id: 25170, stack: 0)
    StoneBrickWall, // 1.14 (id: 29073, stack: 0)
    StoneStairs, // 1.14 (id: 23784, stack: 0)
    SuspiciousStew, // 1.14 (id: 8173, stack: 1)
    SweetBerries, // 1.14 (id: 19747)
    SweetBerryBush, // 1.14 (id: 11958, stack: 0)
    TraderLlamaSpawnEgg, // 1.14 (id: 8439)
    WanderingTraderSpawnEgg, // 1.14 (id: 17904)
    WhiteDye, // 1.14 (id: 10758)
    WitherRose, // 1.14 (id: 8619)
    YellowDye, // 1.14 (id: 5952)
    Beehive, // 1.15 (id: 11830, stack: 0)
    BeeNest, // 1.15 (id: 8825, stack: 0)
    BeeSpawnEgg, // 1.15 (id: 22924)
    Honeycomb, // 1.15 (id: 9482)
    HoneycombBlock, // 1.15 (id: 28780)
    HoneyBlock, // 1.15 (id: 30615)
    HoneyBottle, // 1.15 (id: 22927, stack: 16)
    AncientDebris, // 1.16.1 (id: 18198)
    Basalt, // 1.16.1 (id: 12191, stack: 0)| 1.16.2 (id: 28478)
    Blackstone, // 1.16.1 (id: 7354)
    BlackstoneSlab, // 1.16.1 (id: 11948, stack: 0)
    BlackstoneStairs, // 1.16.1 (id: 14646, stack: 0)
    BlackstoneWall, // 1.16.1 (id: 17327, stack: 0)
    Chain,  // 1.16.1 (id: 28265, stack: 0)
    ChiseledNetherBricks, // 1.16.1 (id: 21613)
    ChiseledPolishedBlackstone, // 1.16.1 (id: 8923)| 1.16.2 (id: 21942)
    CrackedNetherBricks, // 1.16.1 (id: 10888)
    CrackedPolishedBlackstoneBricks, // 1.16.1 (id: 16846)
    CrimsonButton, // 1.16.1 (id: 26799, stack: 0)
    CrimsonDoor, // 1.16.1 (id: 19544, stack: 0)
    CrimsonFence, // 1.16.1 (id: 21075, stack: 0)
    CrimsonFenceGate, // 1.16.1 (id: 15602, stack: 0)
    CrimsonFungus, // 1.16.1 (id: 26268)
    CrimsonHyphae, // 1.16.1 (id: 6550, stack: 0)
    CrimsonNylium, // 1.16.1 (id: 18139)
    CrimsonPlanks, // 1.16.1 (id: 18812)
    CrimsonPressurePlate, // 1.16.1 (id: 18316, stack: 0)
    CrimsonRoots, // 1.16.1 (id: 14064)
    CrimsonSign, // 1.16.1(id; 12162, stack: 16, durability: 0 )
    CrimsonSlab, // 1.16.1 (id: 4691, stack: 0)
    CrimsonStairs, // 1.16.1 (id: 32442, stack: 0)
    CrimsonStem, // 1.16.1 (id: 27920, stack: 0)
    CrimsonTrapdoor, // 1.16.1 (id: 25056, stack: 0)
    CrimsonWallSign, // 1.16.1(id; 7573, stack: 16, durability: 0 )| 1.16.2 (id: 19242)
    CryingObsidian, // 1.16.1 (id: 31545)
    GildedBlackstone, // 1.16.1 (id: 8498)
    HoglinSpawnEgg, // 1.16.1 (id: 14088)
    Lodestone, // 1.16.1 (id: 23127)
    MusicDiscPigstep, // 1.16.1 (id: 21323, stack: 1)
    NetheriteAxe, // 1.16.1(id; 29533, stack: 1, durability: 2031 )
    NetheriteBlock, // 1.16.1 (id: 6527)
    NetheriteBoots, // 1.16.1(id; 21942, stack: 1, durability: 481 )| 1.16.2 (id: 8923)
    NetheriteChestplate, // 1.16.1(id; 6106, stack: 1, durability: 592 )
    NetheriteHelmet, // 1.16.1(id; 15907, stack: 1, durability: 407 )
    NetheriteHoe, // 1.16.1(id; 27385, stack: 1, durability: 2031 )
    NetheriteIngot, // 1.16.1 (id: 32457)
    NetheriteLeggings, // 1.16.1(id; 25605, stack: 1, durability: 555 )
    NetheritePickaxe, // 1.16.1(id; 9930, stack: 1, durability: 2031 )
    NetheriteScrap, // 1.16.1 (id: 29331)
    NetheriteShovel, // 1.16.1(id; 29728, stack: 1, durability: 2031 )
    NetheriteSword, // 1.16.1(id; 23871, stack: 1, durability: 2031 )
    NetherGoldOre, // 1.16.1 (id: 4185)
    NetherSprouts, // 1.16.1 (id: 10431)
    PiglinBannerPattern, // 1.16.1 (id: 22028, stack: 1)
    PiglinSpawnEgg, // 1.16.1 (id: 16193)
    PolishedBasalt, // 1.16.1 (id: 11659, stack: 0)
    PolishedBlackstone, // 1.16.1 (id: 18144)
    PolishedBlackstoneBricks, // 1.16.1 (id: 19844)
    PolishedBlackstoneBrickSlab, // 1.16.1 (id: 12219, stack: 0)
    PolishedBlackstoneBrickStairs, // 1.16.1 (id: 17983, stack: 0)
    PolishedBlackstoneBrickWall, // 1.16.1 (id: 9540, stack: 0)
    PolishedBlackstoneButton, // 1.16.1 (id: 20760, stack: 0)
    PolishedBlackstonePressurePlate, // 1.16.1 (id: 32340, stack: 0)
    PolishedBlackstoneSlab, // 1.16.1 (id: 23430, stack: 0)
    PolishedBlackstoneStairs, // 1.16.1 (id: 8653, stack: 0)
    PolishedBlackstoneWall, // 1.16.1 (id: 15119, stack: 0)
    PottedCrimsonFungus, // 1.16.1 (id: 5548)
    PottedCrimsonRoots, // 1.16.1 (id: 13852)
    PottedWarpedFungus, // 1.16.1 (id: 30800)
    PottedWarpedRoots, // 1.16.1 (id: 6403)
    QuartzBricks, // 1.16.1 (id: 23358)
    RespawnAnchor, // 1.16.1 (id: 4099, stack: 0)
    Shroomlight, // 1.16.1 (id: 20424)
    SoulCampfire, // 1.16.1 (id: 4238, stack: 0)
    SoulFire, // 1.16.1 (id: 30163, stack: 0)
    SoulLantern, // 1.16.1 (id: 27778, stack: 0)
    SoulSoil, // 1.16.1 (id: 31140)
    SoulTorch, // 1.16.1 (id: 14292, stack: 0)
    SoulWallTorch, // 1.16.1 (id: 27500, stack: 0)
    StriderSpawnEgg, // 1.16.1 (id: 6203)
    StrippedCrimsonHyphae, // 1.16.1 (id: 27488, stack: 0)
    StrippedCrimsonStem, // 1.16.1 (id: 16882, stack: 0)
    StrippedWarpedHyphae, // 1.16.1 (id: 7422, stack: 0)
    StrippedWarpedStem, // 1.16.1 (id: 15627, stack: 0)
    Target, // 1.16.1 (id: 22637, stack: 0)
    TwistingVines, // 1.16.1 (id: 27283, stack: 0)
    TwistingVinesPlant, // 1.16.1 (id: 25338)
    WarpedButton, // 1.16.1 (id: 25264, stack: 0)
    WarpedDoor, // 1.16.1 (id: 15062, stack: 0)
    WarpedFence, // 1.16.1 (id: 18438, stack: 0)
    WarpedFenceGate, // 1.16.1 (id: 11115, stack: 0)
    WarpedFungus, // 1.16.1 (id: 19799)
    WarpedFungusOnAStick, // 1.16.1(id; 11706, stack: 1, durability: 100 )
    WarpedHyphae, // 1.16.1 (id: 18439, stack: 0)
    WarpedNylium, // 1.16.1 (id: 26396)
    WarpedPlanks, // 1.16.1 (id: 16045)
    WarpedPressurePlate, // 1.16.1 (id: 29516, stack: 0)
    WarpedRoots, // 1.16.1 (id: 13932)
    WarpedSign, // 1.16.1(id; 10407, stack: 16, durability: 0 )
    WarpedSlab, // 1.16.1 (id: 27150, stack: 0)
    WarpedStairs, // 1.16.1 (id: 17721, stack: 0)
    WarpedStem, // 1.16.1 (id: 28920, stack: 0)
    WarpedTrapdoor, // 1.16.1 (id: 7708, stack: 0)
    WarpedWallSign, // 1.16.1(id; 13534, stack: 16, durability: 0 )
    WarpedWartBlock, // 1.16.1 (id: 15463)
    WeepingVines, // 1.16.1 (id: 29267, stack: 0)
    WeepingVinesPlant, // 1.16.1 (id: 19437)
    ZoglinSpawnEgg, // 1.16.1 (id: 7442)
    ZombifiedPiglinSpawnEgg, // 1.16.1 (id: 6626)
    PiglinBruteSpawnEgg, // 1.16.2 (id: 30230)
    Deepslate, // 1.17 (id: 26842, stack: 0)
    CobbledDeepslate, // 1.17 (id: 8021)
    PolishedDeepslate, // 1.17 (id: 31772)
    Calcite, // 1.17 (id: 20311)
    Tuff,   // 1.17 (id: 24364)
    DripstoneBlock, // 1.17 (id: 26227)
    RootedDirt, // 1.17 (id: 11410)
    DeepslateCoalOre, // 1.17 (id: 16823)
    DeepslateIronOre, // 1.17 (id: 26021)
    CopperOre, // 1.17 (id: 32666)
    DeepslateCopperOre, // 1.17 (id: 6588)
    DeepslateGoldOre, // 1.17 (id: 13582)
    DeepslateRedstoneOre, // 1.17 (id: 6331, stack: 0)
    DeepslateEmeraldOre, // 1.17 (id: 5299)
    DeepslateLapisOre, // 1.17 (id: 13598)
    DeepslateDiamondOre, // 1.17 (id: 17792)
    RawIronBlock, // 1.17 (id: 32210)
    RawCopperBlock, // 1.17 (id: 17504)
    RawGoldBlock, // 1.17 (id: 23246)
    AmethystBlock, // 1.17 (id: 18919)
    BuddingAmethyst, // 1.17 (id: 13963)
    CopperBlock, // 1.17 (id: 12880)
    ExposedCopper, // 1.17 (id: 28488)
    WeatheredCopper, // 1.17 (id: 19699)
    OxidizedCopper, // 1.17 (id: 19490)
    CutCopper, // 1.17 (id: 32519)
    ExposedCutCopper, // 1.17 (id: 18000)
    WeatheredCutCopper, // 1.17 (id: 21158)
    OxidizedCutCopper, // 1.17 (id: 5382)
    CutCopperStairs, // 1.17 (id: 25925, stack: 0)
    ExposedCutCopperStairs, // 1.17 (id: 31621, stack: 0)
    WeatheredCutCopperStairs, // 1.17 (id: 5851, stack: 0)
    OxidizedCutCopperStairs, // 1.17 (id: 25379, stack: 0)
    CutCopperSlab, // 1.17 (id: 28988, stack: 0)
    ExposedCutCopperSlab, // 1.17 (id: 26694, stack: 0)
    WeatheredCutCopperSlab, // 1.17 (id: 4602, stack: 0)
    OxidizedCutCopperSlab, // 1.17 (id: 29642, stack: 0)
    WaxedCopperBlock, // 1.17 (id: 14638)
    WaxedExposedCopper, // 1.17 (id: 27989)
    WaxedWeatheredCopper, // 1.17 (id: 5960)
    WaxedOxidizedCopper, // 1.17 (id: 25626)
    WaxedCutCopper, // 1.17 (id: 11030)
    WaxedExposedCutCopper, // 1.17 (id: 30043)
    WaxedWeatheredCutCopper, // 1.17 (id: 13823)
    WaxedOxidizedCutCopper, // 1.17 (id: 22582)
    WaxedCutCopperStairs, // 1.17 (id: 23125, stack: 0)
    WaxedExposedCutCopperStairs, // 1.17 (id: 15532, stack: 0)
    WaxedWeatheredCutCopperStairs, // 1.17 (id: 29701, stack: 0)
    WaxedOxidizedCutCopperStairs, // 1.17 (id: 9842, stack: 0)
    WaxedCutCopperSlab, // 1.17 (id: 6271, stack: 0)
    WaxedExposedCutCopperSlab, // 1.17 (id: 22091, stack: 0)
    WaxedWeatheredCutCopperSlab, // 1.17 (id: 20035, stack: 0)
    WaxedOxidizedCutCopperSlab, // 1.17 (id: 11202, stack: 0)
    AzaleaLeaves, // 1.17 (id: 23001, stack: 0)
    FloweringAzaleaLeaves, // 1.17 (id: 20893, stack: 0)
    TintedGlass, // 1.17 (id: 19154)
    Azalea, // 1.17 (id: 29386)
    FloweringAzalea, // 1.17 (id: 28270)
    SporeBlossom, // 1.17 (id: 20627)
    MossCarpet, // 1.17 (id: 8221)
    MossBlock, // 1.17 (id: 9175)
    HangingRoots, // 1.17 (id: 15498, stack: 0)
    BigDripleaf, // 1.17 (id: 26173, stack: 0)
    SmallDripleaf, // 1.17 (id: 17540, stack: 0)
    SmoothBasalt, // 1.17 (id: 13617)
    InfestedDeepslate, // 1.17 (id: 9472, stack: 0)
    DeepslateBricks, // 1.17 (id: 13193)
    CrackedDeepslateBricks, // 1.17 (id: 17105)
    DeepslateTiles, // 1.17 (id: 11250)
    CrackedDeepslateTiles, // 1.17 (id: 26249)
    ChiseledDeepslate, // 1.17 (id: 23825)
    GlowLichen, // 1.17 (id: 19165, stack: 0)
    CobbledDeepslateWall, // 1.17 (id: 21893, stack: 0)
    PolishedDeepslateWall, // 1.17 (id: 6574, stack: 0)
    DeepslateBrickWall, // 1.17 (id: 13304, stack: 0)
    DeepslateTileWall, // 1.17 (id: 17077, stack: 0)
    Light,  // 1.17 (id: 17829, stack: 0)
    DirtPath, // 1.17 (id: 10846)
    CobbledDeepslateStairs, // 1.17 (id: 20699, stack: 0)
    PolishedDeepslateStairs, // 1.17 (id: 19513, stack: 0)
    DeepslateBrickStairs, // 1.17 (id: 29624, stack: 0)
    DeepslateTileStairs, // 1.17 (id: 6361, stack: 0)
    CobbledDeepslateSlab, // 1.17 (id: 17388, stack: 0)
    PolishedDeepslateSlab, // 1.17 (id: 32201, stack: 0)
    DeepslateBrickSlab, // 1.17 (id: 23910, stack: 0)
    DeepslateTileSlab, // 1.17 (id: 13315, stack: 0)
    LightningRod, // 1.17 (id: 30770, stack: 0)
    SculkSensor, // 1.17 (id: 5598, stack: 0)
    AmethystShard, // 1.17 (id: 7613)
    RawIron, // 1.17 (id: 5329)
    RawCopper, // 1.17 (id: 6162)
    CopperIngot, // 1.17 (id: 12611)
    RawGold, // 1.17 (id: 19564)
    PowderSnowBucket, // 1.17 (id: 31101, stack: 1)
    AxolotlBucket, // 1.17 (id: 20669, stack: 1)
    Bundle, // 1.17 (id: 16835, stack: 1)
    Spyglass, // 1.17 (id: 27490, stack: 1)
    GlowInkSac, // 1.17 (id: 9686)
    AxolotlSpawnEgg, // 1.17 (id: 30381)
    GlowSquidSpawnEgg, // 1.17 (id: 31578)
    GoatSpawnEgg, // 1.17 (id: 30639)
    GlowItemFrame, // 1.17 (id: 26473)
    GlowBerries, // 1.17 (id: 11584)
    Candle, // 1.17 (id: 16122, stack: 0)
    WhiteCandle, // 1.17 (id: 26410, stack: 0)
    OrangeCandle, // 1.17 (id: 22668, stack: 0)
    MagentaCandle, // 1.17 (id: 25467, stack: 0)
    LightBlueCandle, // 1.17 (id: 28681, stack: 0)
    YellowCandle, // 1.17 (id: 14351, stack: 0)
    LimeCandle, // 1.17 (id: 21778, stack: 0)
    PinkCandle, // 1.17 (id: 28259, stack: 0)
    GrayCandle, // 1.17 (id: 10721, stack: 0)
    LightGrayCandle, // 1.17 (id: 10031, stack: 0)
    CyanCandle, // 1.17 (id: 24765, stack: 0)
    PurpleCandle, // 1.17 (id: 19606, stack: 0)
    BlueCandle, // 1.17 (id: 29047, stack: 0)
    BrownCandle, // 1.17 (id: 26145, stack: 0)
    GreenCandle, // 1.17 (id: 29756, stack: 0)
    RedCandle, // 1.17 (id: 4214, stack: 0)
    BlackCandle, // 1.17 (id: 12617, stack: 0)
    SmallAmethystBud, // 1.17 (id: 14958, stack: 0)
    MediumAmethystBud, // 1.17 (id: 8429, stack: 0)
    LargeAmethystBud, // 1.17 (id: 7279, stack: 0)
    AmethystCluster, // 1.17 (id: 13142, stack: 0)
    PointedDripstone, // 1.17 (id: 18755, stack: 0)
    WaterCauldron, // 1.17 (id: 32008, stack: 0)
    LavaCauldron, // 1.17 (id: 4514, stack: 0)
    PowderSnowCauldron, // 1.17 (id: 31571, stack: 0)
    CandleCake, // 1.17 (id: 25423, stack: 0)
    WhiteCandleCake, // 1.17 (id: 12674, stack: 0)
    OrangeCandleCake, // 1.17 (id: 24982, stack: 0)
    MagentaCandleCake, // 1.17 (id: 11022, stack: 0)
    LightBlueCandleCake, // 1.17 (id: 7787, stack: 0)
    YellowCandleCake, // 1.17 (id: 17157, stack: 0)
    LimeCandleCake, // 1.17 (id: 14309, stack: 0)
    PinkCandleCake, // 1.17 (id: 20405, stack: 0)
    GrayCandleCake, // 1.17 (id: 6777, stack: 0)
    LightGrayCandleCake, // 1.17 (id: 11318, stack: 0)
    CyanCandleCake, // 1.17 (id: 21202, stack: 0)
    PurpleCandleCake, // 1.17 (id: 22663, stack: 0)
    BlueCandleCake, // 1.17 (id: 26425, stack: 0)
    BrownCandleCake, // 1.17 (id: 26024, stack: 0)
    GreenCandleCake, // 1.17 (id: 16334, stack: 0)
    RedCandleCake, // 1.17 (id: 24151, stack: 0)
    BlackCandleCake, // 1.17 (id: 15191, stack: 0)
    PowderSnow, // 1.17 (id: 24077)
    CaveVines, // 1.17 (id: 7339, stack: 0)
    CaveVinesPlant, // 1.17 (id: 30645, stack: 0)
    BigDripleafStem, // 1.17 (id: 13167, stack: 0)
    PottedAzaleaBush, // 1.17 (id: 20430)
    PottedFloweringAzaleaBush, // 1.17 (id: 10609)
}

impl Material {
    pub fn name(&self) -> String {
        format!("{:?}", self)
    }

    pub fn texture_locations(&self) -> (String, String) {
        // TODO: Compute this at compile time and only lookup at runtime in (O(1))
        let mut result = String::new();
        for (i, c) in self.name().chars().enumerate() {
            if c.is_uppercase() {
                if i != 0 {
                    result.push('_');
                }
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        }
        (format!("item/{}", result), format!("block/{}", result))
    }

    pub fn as_tool(&self) -> Option<block::Tool> {
        use block::{Tool, ToolMaterial};
        match *self {
            Material::WoodPickaxe => Some(Tool::Pickaxe(ToolMaterial::Wooden)),
            Material::StonePickaxe => Some(Tool::Pickaxe(ToolMaterial::Stone)),
            Material::GoldPickaxe => Some(Tool::Pickaxe(ToolMaterial::Golden)),
            Material::IronPickaxe => Some(Tool::Pickaxe(ToolMaterial::Iron)),
            Material::DiamondPickaxe => Some(Tool::Pickaxe(ToolMaterial::Diamond)),
            Material::NetheritePickaxe => Some(Tool::Pickaxe(ToolMaterial::Netherite)),

            Material::WoodAxe => Some(Tool::Axe(ToolMaterial::Wooden)),
            Material::StoneAxe => Some(Tool::Axe(ToolMaterial::Stone)),
            Material::GoldAxe => Some(Tool::Axe(ToolMaterial::Golden)),
            Material::IronAxe => Some(Tool::Axe(ToolMaterial::Iron)),
            Material::DiamondAxe => Some(Tool::Axe(ToolMaterial::Diamond)),
            Material::NetheriteAxe => Some(Tool::Axe(ToolMaterial::Netherite)),

            Material::WoodSpade => Some(Tool::Shovel(ToolMaterial::Wooden)),
            Material::StoneSpade => Some(Tool::Shovel(ToolMaterial::Stone)),
            Material::GoldSpade => Some(Tool::Shovel(ToolMaterial::Golden)),
            Material::IronSpade => Some(Tool::Shovel(ToolMaterial::Iron)),
            Material::DiamondSpade => Some(Tool::Shovel(ToolMaterial::Diamond)),
            Material::NetheriteShovel => Some(Tool::Shovel(ToolMaterial::Netherite)),

            Material::WoodHoe => Some(Tool::Hoe(ToolMaterial::Wooden)),
            Material::StoneHoe => Some(Tool::Hoe(ToolMaterial::Stone)),
            Material::GoldHoe => Some(Tool::Hoe(ToolMaterial::Golden)),
            Material::IronHoe => Some(Tool::Hoe(ToolMaterial::Iron)),
            Material::DiamondHoe => Some(Tool::Hoe(ToolMaterial::Diamond)),
            Material::NetheriteHoe => Some(Tool::Hoe(ToolMaterial::Netherite)),

            Material::WoodSword => Some(Tool::Sword(ToolMaterial::Wooden)),
            Material::StoneSword => Some(Tool::Sword(ToolMaterial::Stone)),
            Material::GoldSword => Some(Tool::Sword(ToolMaterial::Golden)),
            Material::IronSword => Some(Tool::Sword(ToolMaterial::Iron)),
            Material::DiamondSword => Some(Tool::Sword(ToolMaterial::Diamond)),
            Material::NetheriteSword => Some(Tool::Sword(ToolMaterial::Netherite)),

            Material::Shears => Some(Tool::Shears),
            _ => None,
        }
    }

    pub fn get_stack_size(&self, version: Version) -> u8 {
        material::versions::get_stack_size(*self, version)
    }

    pub fn is_placable_block(&self, version: Version, id: isize) -> bool {
        if version < Version::V1_13 {
            return id < 256;
        }

        // FIXME: add support for 1.13+
        true
    }

    /*
    pub fn is_placable_block(&self) -> bool {
        match self {
            Material::AcaciaButton |
            Material::AcaciaDoorItem |
            Material::AcaciaFence |
            Material::AcaciaFenceGate |
            Material::AcaciaLeaves |
            Material::AcaciaLog |
            Material::AcaciaPlanks |
            Material::AcaciaPressurePlate |
            Material::AcaciaSapling |
            Material::AcaciaSign |
            Material::AcaciaSlab |
            Material::AcaciaStairs |
            Material::AcaciaTrapdoor |
            Material::AcaciaWood |
            Material::ActivatorRail |
            Material::Allium |
            Material::AmethystBlock |
            Material::AmethystCluster |
            Material::AmethystShard |
            Material::AncientDebris |
            Material::Andesite |
            Material::AndesiteSlab |
            Material::AndesiteStairs |
            Material::AndesiteWall |
            Material::Anvil |
            Material::Azalea |
            Material::AzaleaLeaves |
            Material::AzureBluet |
            Material::Bamboo |
            Material::Banner |
            Material::Barrel |
            Material::Barrier |
            Material::Basalt |
            Material::Beacon |
            Material::Bed |
            Material::Bedrock |
            Material::BeeNest |
            Material::Beehive |
            Material::Bell |
            Material::BigDripleaf |
            Material::BirchButton |
            Material::BirchDoorItem |
            Material::BirchFence |
            Material::BirchFenceGate |
            Material::BirchLeaves |
            Material::BirchLog |
            Material::BirchPlanks |
            Material::BirchPressurePlate |
            Material::BirchSapling |
            Material::BirchSign |
            Material::BirchSlab |
            Material::BirchStairs |
            Material::BirchTrapdoor |
            Material::BirchWood |
            Material::BirchWoodStairs |
            Material::BlackBanner |
            Material::BlackBed |
            Material::BlackCandle |
            Material::BlackCarpet |
            Material::BlackConcrete |
            Material::BlackConcretePowder |
            Material::BlackGlazedTerracotta
        }
    }
    */
}
