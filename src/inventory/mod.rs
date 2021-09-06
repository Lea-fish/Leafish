pub mod player_inventory;

use crate::inventory::player_inventory::PlayerInventory;
use crate::render::hud::HudContext;
use crate::render::inventory::InventoryWindow;
use crate::render::Renderer;
use crate::ui::Container;
use leafish_protocol::item::Stack;
use leafish_protocol::protocol::Version;
use log::info;
use parking_lot::RwLock;
use std::sync::Arc;

pub trait Inventory {
    fn size(&self) -> i16;

    fn id(&self) -> i8;

    fn name(&self) -> Option<&String>;

    fn get_item(&self, slot: i16) -> &Option<Item>;

    fn get_item_mut(&mut self, slot: i16) -> &mut Option<Item>;

    fn set_item(&mut self, slot: i16, item: Option<Item>);

    fn init(
        &mut self,
        renderer: &mut Renderer,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    );

    fn tick(
        &mut self,
        renderer: &mut Renderer,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    );

    fn close(&mut self, inventory_window: &mut InventoryWindow);

    fn click_at(&self, cursor: (u32, u32)); // TODO: Pass mouse data (buttons, wheel etc and shift button state)

    fn resize(
        &mut self,
        width: u32,
        height: u32,
        renderer: &mut Renderer,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
    );

    fn ty(&self) -> InventoryType;
}

pub struct Slot {
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub item: Option<Item>,
    // TODO: Is valid fn for Anvil, crafting, armor etc.
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
}

pub struct InventoryContext {
    pub cursor: Option<Item>,
    pub hotbar_index: u8,
    pub inventory: Option<Arc<RwLock<dyn Inventory + Send + Sync>>>,
    pub player_inventory: Arc<RwLock<PlayerInventory>>,
}

impl InventoryContext {
    pub fn new(
        version: Version,
        renderer: &Renderer,
        hud_context: Arc<RwLock<HudContext>>,
    ) -> Self {
        InventoryContext {
            cursor: None,
            hotbar_index: 0,
            inventory: None,
            player_inventory: Arc::new(RwLock::new(PlayerInventory::new(
                version,
                renderer,
                hud_context,
            ))),
        }
    }
}

pub enum InventoryType {
    Main,
    Chest,
    Hopper,
    Enchanter,
    Anvil,
    Beacon,
    Brewer,
    CraftingTable,
    Dropper,
    Horse,
    Merchant,
    EntityEquipment,
}

#[derive(Debug)]
pub struct Item {
    pub stack: Stack,
    pub material: Material,
}

#[derive(Debug)]
pub enum Material {
    Air,                      // 1.8.8 (id: 0, stack: 0)
    Stone,                    // 1.8.8 (id: 1)
    Grass,                    // 1.8.8 (id: 2)
    Dirt,                     // 1.8.8 (id: 3)
    Cobblestone,              // 1.8.8 (id: 4)
    Wood,                     // 1.8.8 (id: 5, class: Tree)
    Sapling,                  // 1.8.8 (id: 6, class: Tree)
    Bedrock,                  // 1.8.8 (id: 7)
    Water,                    // 1.8.8 (id: 8, class: MaterialData)
    StationaryWater,          // 1.8.8 (id: 9, class: MaterialData)
    Lava,                     // 1.8.8 (id: 10, class: MaterialData)
    StationaryLava,           // 1.8.8 (id: 11, class: MaterialData)
    Sand,                     // 1.8.8 (id: 12)
    Gravel,                   // 1.8.8 (id: 13)
    GoldOre,                  // 1.8.8 (id: 14)
    IronOre,                  // 1.8.8 (id: 15)
    CoalOre,                  // 1.8.8 (id: 16)
    Log,                      // 1.8.8 (id: 17, class: Tree)
    Leaves,                   // 1.8.8 (id: 18, class: Tree)
    Sponge,                   // 1.8.8 (id: 19)
    Glass,                    // 1.8.8 (id: 20)
    LapisOre,                 // 1.8.8 (id: 21)
    LapisBlock,               // 1.8.8 (id: 22)
    Dispenser,                // 1.8.8 (id: 23, class: Dispenser)
    Sandstone,                // 1.8.8 (id: 24, class: Sandstone)
    NoteBlock,                // 1.8.8 (id: 25)
    BedBlock,                 // 1.8.8 (id: 26, class: Bed)
    PoweredRail,              // 1.8.8 (id: 27, class: PoweredRail)
    DetectorRail,             // 1.8.8 (id: 28, class: DetectorRail)
    PistonStickyBase,         // 1.8.8 (id: 29, class: PistonBaseMaterial)
    Web,                      // 1.8.8 (id: 30)
    LongGrass,                // 1.8.8 (id: 31, class: LongGrass)
    DeadBush,                 // 1.8.8 (id: 32)
    PistonBase,               // 1.8.8 (id: 33, class: PistonBaseMaterial)
    PistonExtension,          // 1.8.8 (id: 34, class: PistonExtensionMaterial)
    Wool,                     // 1.8.8 (id: 35, class: Wool)
    PistonMovingPiece,        // 1.8.8 (id: 36)
    YellowFlower,             // 1.8.8 (id: 37)
    RedRose,                  // 1.8.8 (id: 38)
    BrownMushroom,            // 1.8.8 (id: 39)
    RedMushroom,              // 1.8.8 (id: 40)
    GoldBlock,                // 1.8.8 (id: 41)
    IronBlock,                // 1.8.8 (id: 42)
    DoubleStep,               // 1.8.8 (id: 43, class: Step)
    Step,                     // 1.8.8 (id: 44, class: Step)
    Brick,                    // 1.8.8 (id: 45)
    Tnt,                      // 1.8.8 (id: 46)
    Bookshelf,                // 1.8.8 (id: 47)
    MossyCobblestone,         // 1.8.8 (id: 48)
    Obsidian,                 // 1.8.8 (id: 49)
    Torch,                    // 1.8.8 (id: 50, class: Torch)
    Fire,                     // 1.8.8 (id: 51)
    MobSpawner,               // 1.8.8 (id: 52)
    WoodStairs,               // 1.8.8 (id: 53, class: Stairs)
    Chest,                    // 1.8.8 (id: 54, class: Chest)
    RedstoneWire,             // 1.8.8 (id: 55, class: RedstoneWire)
    DiamondOre,               // 1.8.8 (id: 56)
    DiamondBlock,             // 1.8.8 (id: 57)
    Workbench,                // 1.8.8 (id: 58)
    Crops,                    // 1.8.8 (id: 59, class: Crops)
    Soil,                     // 1.8.8 (id: 60, class: MaterialData)
    Furnace,                  // 1.8.8 (id: 61, class: Furnace)
    BurningFurnace,           // 1.8.8 (id: 62, class: Furnace)
    SignPost,                 // 1.8.8 (id: 63, stack: 64, class: Sign)
    WoodenDoor,               // 1.8.8 (id: 64, class: Door)
    Ladder,                   // 1.8.8 (id: 65, class: Ladder)
    Rails,                    // 1.8.8 (id: 66, class: Rails)
    CobblestoneStairs,        // 1.8.8 (id: 67, class: Stairs)
    WallSign,                 // 1.8.8 (id: 68, stack: 64, class: Sign)
    Lever,                    // 1.8.8 (id: 69, class: Lever)
    StonePlate,               // 1.8.8 (id: 70, class: PressurePlate)
    IronDoorBlock,            // 1.8.8 (id: 71, class: Door)
    WoodPlate,                // 1.8.8 (id: 72, class: PressurePlate)
    RedstoneOre,              // 1.8.8 (id: 73)
    GlowingRedstoneOre,       // 1.8.8 (id: 74)
    RedstoneTorchOff,         // 1.8.8 (id: 75, class: RedstoneTorch)
    RedstoneTorchOn,          // 1.8.8 (id: 76, class: RedstoneTorch)
    StoneButton,              // 1.8.8 (id: 77, class: Button)
    Snow,                     // 1.8.8 (id: 78)
    Ice,                      // 1.8.8 (id: 79)
    SnowBlock,                // 1.8.8 (id: 80)
    Cactus,                   // 1.8.8 (id: 81, class: MaterialData)
    Clay,                     // 1.8.8 (id: 82)
    SugarCaneBlock,           // 1.8.8 (id: 83, class: MaterialData)
    Jukebox,                  // 1.8.8 (id: 84)
    Fence,                    // 1.8.8 (id: 85)
    Pumpkin,                  // 1.8.8 (id: 86, class: Pumpkin)
    Netherrack,               // 1.8.8 (id: 87)
    SoulSand,                 // 1.8.8 (id: 88)
    Glowstone,                // 1.8.8 (id: 89)
    Portal,                   // 1.8.8 (id: 90)
    JackOLantern,             // 1.8.8 (id: 91, class: Pumpkin)
    CakeBlock,                // 1.8.8 (id: 92, stack: 64, class: Cake)
    DiodeBlockOff,            // 1.8.8 (id: 93, class: Diode)
    DiodeBlockOn,             // 1.8.8 (id: 94, class: Diode)
    StainedGlass,             // 1.8.8 (id: 95)
    TrapDoor,                 // 1.8.8 (id: 96, class: TrapDoor)
    MonsterEggs,              // 1.8.8 (id: 97, class: MonsterEggs)
    SmoothBrick,              // 1.8.8 (id: 98, class: SmoothBrick)
    HugeMushroom1,            // 1.8.8 (id: 99, class: Mushroom)
    HugeMushroom2,            // 1.8.8 (id: 100, class: Mushroom)
    IronFence,                // 1.8.8 (id: 101)
    ThinGlass,                // 1.8.8 (id: 102)
    MelonBlock,               // 1.8.8 (id: 103)
    PumpkinStem,              // 1.8.8 (id: 104, class: MaterialData)
    MelonStem,                // 1.8.8 (id: 105, class: MaterialData)
    Vine,                     // 1.8.8 (id: 106, class: Vine)
    FenceGate,                // 1.8.8 (id: 107, class: Gate)
    BrickStairs,              // 1.8.8 (id: 108, class: Stairs)
    SmoothStairs,             // 1.8.8 (id: 109, class: Stairs)
    Mycel,                    // 1.8.8 (id: 110)
    WaterLily,                // 1.8.8 (id: 111)
    NetherBrick,              // 1.8.8 (id: 112)
    NetherFence,              // 1.8.8 (id: 113)
    NetherBrickStairs,        // 1.8.8 (id: 114, class: Stairs)
    NetherWarts,              // 1.8.8 (id: 115, class: NetherWarts)
    EnchantmentTable,         // 1.8.8 (id: 116)
    BrewingStand,             // 1.8.8 (id: 117, class: MaterialData)
    Cauldron,                 // 1.8.8 (id: 118, class: Cauldron)
    EnderPortal,              // 1.8.8 (id: 119)
    EnderPortalFrame,         // 1.8.8 (id: 120)
    EnderStone,               // 1.8.8 (id: 121)
    DragonEgg,                // 1.8.8 (id: 122)
    RedstoneLampOff,          // 1.8.8 (id: 123)
    RedstoneLampOn,           // 1.8.8 (id: 124)
    WoodDoubleStep,           // 1.8.8 (id: 125, class: WoodenStep)
    WoodStep,                 // 1.8.8 (id: 126, class: WoodenStep)
    Cocoa,                    // 1.8.8 (id: 127, class: CocoaPlant)
    SandstoneStairs,          // 1.8.8 (id: 128, class: Stairs)
    EmeraldOre,               // 1.8.8 (id: 129)
    EnderChest,               // 1.8.8 (id: 130, class: EnderChest)
    TripwireHook,             // 1.8.8 (id: 131, class: TripwireHook)
    Tripwire,                 // 1.8.8 (id: 132, class: Tripwire)
    EmeraldBlock,             // 1.8.8 (id: 133)
    SpruceWoodStairs,         // 1.8.8 (id: 134, class: Stairs)
    BirchWoodStairs,          // 1.8.8 (id: 135, class: Stairs)
    JungleWoodStairs,         // 1.8.8 (id: 136, class: Stairs)
    Command,                  // 1.8.8 (id: 137, class: Command)
    Beacon,                   // 1.8.8 (id: 138)
    CobbleWall,               // 1.8.8 (id: 139)
    FlowerPot,                // 1.8.8 (id: 140, class: FlowerPot)
    Carrot,                   // 1.8.8 (id: 141)
    Potato,                   // 1.8.8 (id: 142)
    WoodButton,               // 1.8.8 (id: 143, class: Button)
    Skull,                    // 1.8.8 (id: 144, class: Skull)
    Anvil,                    // 1.8.8 (id: 145)
    TrappedChest,             // 1.8.8 (id: 146, class: Chest)
    GoldPlate,                // 1.8.8 (id: 147)
    IronPlate,                // 1.8.8 (id: 148)
    RedstoneComparatorOff,    // 1.8.8 (id: 149)
    RedstoneComparatorOn,     // 1.8.8 (id: 150)
    DaylightDetector,         // 1.8.8 (id: 151)
    RedstoneBlock,            // 1.8.8 (id: 152)
    QuartzOre,                // 1.8.8 (id: 153)
    Hopper,                   // 1.8.8 (id: 154)
    QuartzBlock,              // 1.8.8 (id: 155)
    QuartzStairs,             // 1.8.8 (id: 156, class: Stairs)
    ActivatorRail,            // 1.8.8 (id: 157, class: PoweredRail)
    Dropper,                  // 1.8.8 (id: 158, class: Dispenser)
    StainedClay,              // 1.8.8 (id: 159)
    StainedGlassPane,         // 1.8.8 (id: 160)
    Leaves2,                  // 1.8.8 (id: 161)
    Log2,                     // 1.8.8 (id: 162)
    AcaciaStairs,             // 1.8.8 (id: 163, class: Stairs)
    DarkOakStairs,            // 1.8.8 (id: 164, class: Stairs)
    SlimeBlock,               // 1.8.8 (id: 165)
    Barrier,                  // 1.8.8 (id: 166)
    IronTrapdoor,             // 1.8.8 (id: 167, class: TrapDoor)
    Prismarine,               // 1.8.8 (id: 168)
    SeaLantern,               // 1.8.8 (id: 169)
    HayBlock,                 // 1.8.8 (id: 170)
    Carpet,                   // 1.8.8 (id: 171)
    HardClay,                 // 1.8.8 (id: 172)
    CoalBlock,                // 1.8.8 (id: 173)
    PackedIce,                // 1.8.8 (id: 174)
    DoublePlant,              // 1.8.8 (id: 175)
    StandingBanner,           // 1.8.8 (id: 176, class: Banner)
    WallBanner,               // 1.8.8 (id: 177, class: Banner)
    DaylightDetectorInverted, // 1.8.8 (id: 178)
    RedSandstone,             // 1.8.8 (id: 179)
    RedSandstoneStairs,       // 1.8.8 (id: 180, class: Stairs)
    DoubleStoneSlab2,         // 1.8.8 (id: 181)
    StoneSlab2,               // 1.8.8 (id: 182)
    SpruceFenceGate,          // 1.8.8 (id: 183, class: Gate)
    BirchFenceGate,           // 1.8.8 (id: 184, class: Gate)
    JungleFenceGate,          // 1.8.8 (id: 185, class: Gate)
    DarkOakFenceGate,         // 1.8.8 (id: 186, class: Gate)
    AcaciaFenceGate,          // 1.8.8 (id: 187, class: Gate)
    SpruceFence,              // 1.8.8 (id: 188)
    BirchFence,               // 1.8.8 (id: 189)
    JungleFence,              // 1.8.8 (id: 190)
    DarkOakFence,             // 1.8.8 (id: 191)
    AcaciaFence,              // 1.8.8 (id: 192)
    SpruceDoor,               // 1.8.8 (id: 193, class: Door)
    BirchDoor,                // 1.8.8 (id: 194, class: Door)
    JungleDoor,               // 1.8.8 (id: 195, class: Door)
    AcaciaDoor,               // 1.8.8 (id: 196, class: Door)
    DarkOakDoor,              // 1.8.8 (id: 197, class: Door)
    IronSpade,                // 1.8.8 (id: 256, stack: 1, durability: 250)
    IronPickaxe,              // 1.8.8 (id: 257, stack: 1, durability: 250)
    IronAxe,                  // 1.8.8 (id: 258, stack: 1, durability: 250)
    FlintAndSteel,            // 1.8.8 (id: 259, stack: 1, durability: 64)
    Apple,                    // 1.8.8 (id: 260)
    Bow,                      // 1.8.8 (id: 261, stack: 1, durability: 384)
    Arrow,                    // 1.8.8 (id: 262)
    Coal,                     // 1.8.8 (id: 263, class: Coal)
    Diamond,                  // 1.8.8 (id: 264)
    IronIngot,                // 1.8.8 (id: 265)
    GoldIngot,                // 1.8.8 (id: 266)
    IronSword,                // 1.8.8 (id: 267, stack: 1, durability: 250)
    WoodSword,                // 1.8.8 (id: 268, stack: 1, durability: 59)
    WoodSpade,                // 1.8.8 (id: 269, stack: 1, durability: 59)
    WoodPickaxe,              // 1.8.8 (id: 270, stack: 1, durability: 59)
    WoodAxe,                  // 1.8.8 (id: 271, stack: 1, durability: 59)
    StoneSword,               // 1.8.8 (id: 272, stack: 1, durability: 131)
    StoneSpade,               // 1.8.8 (id: 273, stack: 1, durability: 131)
    StonePickaxe,             // 1.8.8 (id: 274, stack: 1, durability: 131)
    StoneAxe,                 // 1.8.8 (id: 275, stack: 1, durability: 131)
    DiamondSword,             // 1.8.8 (id: 276, stack: 1, durability: 1561)
    DiamondSpade,             // 1.8.8 (id: 277, stack: 1, durability: 1561)
    DiamondPickaxe,           // 1.8.8 (id: 278, stack: 1, durability: 1561)
    DiamondAxe,               // 1.8.8 (id: 279, stack: 1, durability: 1561)
    Stick,                    // 1.8.8 (id: 280)
    Bowl,                     // 1.8.8 (id: 281)
    MushroomSoup,             // 1.8.8 (id: 282, stack: 1)
    GoldSword,                // 1.8.8 (id: 283, stack: 1, durability: 32)
    GoldSpade,                // 1.8.8 (id: 284, stack: 1, durability: 32)
    GoldPickaxe,              // 1.8.8 (id: 285, stack: 1, durability: 32)
    GoldAxe,                  // 1.8.8 (id: 286, stack: 1, durability: 32)
    String,                   // 1.8.8 (id: 287)
    Feather,                  // 1.8.8 (id: 288)
    Sulphur,                  // 1.8.8 (id: 289)
    WoodHoe,                  // 1.8.8 (id: 290, stack: 1, durability: 59)
    StoneHoe,                 // 1.8.8 (id: 291, stack: 1, durability: 131)
    IronHoe,                  // 1.8.8 (id: 292, stack: 1, durability: 250)
    DiamondHoe,               // 1.8.8 (id: 293, stack: 1, durability: 1561)
    GoldHoe,                  // 1.8.8 (id: 294, stack: 1, durability: 32)
    Seeds,                    // 1.8.8 (id: 295)
    Wheat,                    // 1.8.8 (id: 296)
    Bread,                    // 1.8.8 (id: 297)
    LeatherHelmet,            // 1.8.8 (id: 298, stack: 1, durability: 55)
    LeatherChestplate,        // 1.8.8 (id: 299, stack: 1, durability: 80)
    LeatherLeggings,          // 1.8.8 (id: 300, stack: 1, durability: 75)
    LeatherBoots,             // 1.8.8 (id: 301, stack: 1, durability: 65)
    ChainmailHelmet,          // 1.8.8 (id: 302, stack: 1, durability: 165)
    ChainmailChestplate,      // 1.8.8 (id: 303, stack: 1, durability: 240)
    ChainmailLeggings,        // 1.8.8 (id: 304, stack: 1, durability: 225)
    ChainmailBoots,           // 1.8.8 (id: 305, stack: 1, durability: 195)
    IronHelmet,               // 1.8.8 (id: 306, stack: 1, durability: 165)
    IronChestplate,           // 1.8.8 (id: 307, stack: 1, durability: 240)
    IronLeggings,             // 1.8.8 (id: 308, stack: 1, durability: 225)
    IronBoots,                // 1.8.8 (id: 309, stack: 1, durability: 195)
    DiamondHelmet,            // 1.8.8 (id: 310, stack: 1, durability: 363)
    DiamondChestplate,        // 1.8.8 (id: 311, stack: 1, durability: 528)
    DiamondLeggings,          // 1.8.8 (id: 312, stack: 1, durability: 495)
    DiamondBoots,             // 1.8.8 (id: 313, stack: 1, durability: 429)
    GoldHelmet,               // 1.8.8 (id: 314, stack: 1, durability: 77)
    GoldChestplate,           // 1.8.8 (id: 315, stack: 1, durability: 112)
    GoldLeggings,             // 1.8.8 (id: 316, stack: 1, durability: 105)
    GoldBoots,                // 1.8.8 (id: 317, stack: 1, durability: 91)
    Flint,                    // 1.8.8 (id: 318)
    Pork,                     // 1.8.8 (id: 319)
    GrilledPork,              // 1.8.8 (id: 320)
    Painting,                 // 1.8.8 (id: 321)
    GoldenApple,              // 1.8.8 (id: 322)
    Sign,                     // 1.8.8 (id: 323, stack: 16)
    WoodDoor,                 // 1.8.8 (id: 324, stack: 64)
    Bucket,                   // 1.8.8 (id: 325, stack: 16)
    WaterBucket,              // 1.8.8 (id: 326, stack: 1)
    LavaBucket,               // 1.8.8 (id: 327, stack: 1)
    Minecart,                 // 1.8.8 (id: 328, stack: 1)
    Saddle,                   // 1.8.8 (id: 329, stack: 1)
    IronDoor,                 // 1.8.8 (id: 330, stack: 64)
    Redstone,                 // 1.8.8 (id: 331)
    SnowBall,                 // 1.8.8 (id: 332, stack: 16)
    Boat,                     // 1.8.8 (id: 333, stack: 1)
    Leather,                  // 1.8.8 (id: 334)
    MilkBucket,               // 1.8.8 (id: 335, stack: 1)
    ClayBrick,                // 1.8.8 (id: 336)
    ClayBall,                 // 1.8.8 (id: 337)
    SugarCane,                // 1.8.8 (id: 338)
    Paper,                    // 1.8.8 (id: 339)
    Book,                     // 1.8.8 (id: 340)
    SlimeBall,                // 1.8.8 (id: 341)
    StorageMinecart,          // 1.8.8 (id: 342, stack: 1)
    PoweredMinecart,          // 1.8.8 (id: 343, stack: 1)
    Egg,                      // 1.8.8 (id: 344, stack: 16)
    Compass,                  // 1.8.8 (id: 345)
    FishingRod,               // 1.8.8 (id: 346, stack: 1, durability: 64)
    Watch,                    // 1.8.8 (id: 347)
    GlowstoneDust,            // 1.8.8 (id: 348)
    RawFish,                  // 1.8.8 (id: 349)
    CookedFish,               // 1.8.8 (id: 350)
    InkSack,                  // 1.8.8 (id: 351, class: Dye)
    Bone,                     // 1.8.8 (id: 352)
    Sugar,                    // 1.8.8 (id: 353)
    Cake,                     // 1.8.8 (id: 354, stack: 1)
    Bed,                      // 1.8.8 (id: 355, stack: 1)
    Diode,                    // 1.8.8 (id: 356)
    Cookie,                   // 1.8.8 (id: 357)
    Map,                      // 1.8.8 (id: 358, class: MaterialData)
    Shears,                   // 1.8.8 (id: 359, stack: 1, durability: 238)
    Melon,                    // 1.8.8 (id: 360)
    PumpkinSeeds,             // 1.8.8 (id: 361)
    MelonSeeds,               // 1.8.8 (id: 362)
    RawBeef,                  // 1.8.8 (id: 363)
    CookedBeef,               // 1.8.8 (id: 364)
    RawChicken,               // 1.8.8 (id: 365)
    CookedChicken,            // 1.8.8 (id: 366)
    RottenFlesh,              // 1.8.8 (id: 367)
    EnderPearl,               // 1.8.8 (id: 368, stack: 16)
    BlazeRod,                 // 1.8.8 (id: 369)
    GhastTear,                // 1.8.8 (id: 370)
    GoldNugget,               // 1.8.8 (id: 371)
    NetherStalk,              // 1.8.8 (id: 372)
    Potion,                   // 1.8.8 (id: 373, stack: 1, class: MaterialData)
    GlassBottle,              // 1.8.8 (id: 374)
    SpiderEye,                // 1.8.8 (id: 375)
    FermentedSpiderEye,       // 1.8.8 (id: 376)
    BlazePowder,              // 1.8.8 (id: 377)
    MagmaCream,               // 1.8.8 (id: 378)
    BrewingStandItem,         // 1.8.8 (id: 379)
    CauldronItem,             // 1.8.8 (id: 380)
    EyeOfEnder,               // 1.8.8 (id: 381)
    SpeckledMelon,            // 1.8.8 (id: 382)
    MonsterEgg,               // 1.8.8 (id: 383, stack: 64, class: SpawnEgg)
    ExpBottle,                // 1.8.8 (id: 384, stack: 64)
    Fireball,                 // 1.8.8 (id: 385, stack: 64)
    BookAndQuill,             // 1.8.8 (id: 386, stack: 1)
    WrittenBook,              // 1.8.8 (id: 387, stack: 16)
    Emerald,                  // 1.8.8 (id: 388, stack: 64)
    ItemFrame,                // 1.8.8 (id: 389)
    FlowerPotItem,            // 1.8.8 (id: 390)
    CarrotItem,               // 1.8.8 (id: 391)
    PotatoItem,               // 1.8.8 (id: 392)
    BakedPotato,              // 1.8.8 (id: 393)
    PoisonousPotato,          // 1.8.8 (id: 394)
    EmptyMap,                 // 1.8.8 (id: 395)
    GoldenCarrot,             // 1.8.8 (id: 396)
    SkullItem,                // 1.8.8 (id: 397)
    CarrotStick,              // 1.8.8 (id: 398, stack: 1, durability: 25)
    NetherStar,               // 1.8.8 (id: 399)
    PumpkinPie,               // 1.8.8 (id: 400)
    Firework,                 // 1.8.8 (id: 401)
    FireworkCharge,           // 1.8.8 (id: 402)
    EnchantedBook,            // 1.8.8 (id: 403, stack: 1)
    RedstoneComparator,       // 1.8.8 (id: 404)
    NetherBrickItem,          // 1.8.8 (id: 405)
    Quartz,                   // 1.8.8 (id: 406)
    ExplosiveMinecart,        // 1.8.8 (id: 407, stack: 1)
    HopperMinecart,           // 1.8.8 (id: 408, stack: 1)
    PrismarineShard,          // 1.8.8 (id: 409)
    PrismarineCrystals,       // 1.8.8 (id: 410)
    Rabbit,                   // 1.8.8 (id: 411)
    CookedRabbit,             // 1.8.8 (id: 412)
    RabbitStew,               // 1.8.8 (id: 413, stack: 1)
    RabbitFoot,               // 1.8.8 (id: 414)
    RabbitHide,               // 1.8.8 (id: 415)
    ArmorStand,               // 1.8.8 (id: 416, stack: 16)
    IronBarding,              // 1.8.8 (id: 417, stack: 1)
    GoldBarding,              // 1.8.8 (id: 418, stack: 1)
    DiamondBarding,           // 1.8.8 (id: 419, stack: 1)
    Leash,                    // 1.8.8 (id: 420)
    NameTag,                  // 1.8.8 (id: 421)
    CommandMinecart,          // 1.8.8 (id: 422, stack: 1)
    Mutton,                   // 1.8.8 (id: 423)
    CookedMutton,             // 1.8.8 (id: 424)
    Banner,                   // 1.8.8 (id: 425, stack: 16)
    SpruceDoorItem,           // 1.8.8 (id: 427)
    BirchDoorItem,            // 1.8.8 (id: 428)
    JungleDoorItem,           // 1.8.8 (id: 429)
    AcaciaDoorItem,           // 1.8.8 (id: 430)
    DarkOakDoorItem,          // 1.8.8 (id: 431)
    GoldRecord,               // 1.8.8 (id: 2256, stack: 1)
    GreenRecord,              // 1.8.8 (id: 2257, stack: 1)
    Record3,                  // 1.8.8 (id: 2258, stack: 1)
    Record4,                  // 1.8.8 (id: 2259, stack: 1)
    Record5,                  // 1.8.8 (id: 2260, stack: 1)
    Record6,                  // 1.8.8 (id: 2261, stack: 1)
    Record7,                  // 1.8.8 (id: 2262, stack: 1)
    Record8,                  // 1.8.8 (id: 2263, stack: 1)
    Record9,                  // 1.8.8 (id: 2264, stack: 1)
    Record10,                 // 1.8.8 (id: 2265, stack: 1)
    Record11,                 // 1.8.8 (id: 2266, stack: 1)
    Record12,                 // 1.8.8 (id: 2267, stack: 1)
}

impl Material {
    pub fn name(&self) -> String {
        format!("{:?}", self)
    }

    pub fn texture_location(&self) -> String {
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
        format!("items/{}", result)
    }
}

impl From<isize> for Material {
    fn from(id: isize) -> Self {
        info!("Trying to get material with id: {}", id);
        match id {
            0 => Material::Air,
            1 => Material::Stone,
            2 => Material::Grass,
            3 => Material::Dirt,
            4 => Material::Cobblestone,
            5 => Material::Wood,
            6 => Material::Sapling,
            7 => Material::Bedrock,
            8 => Material::Water,
            9 => Material::StationaryWater,
            10 => Material::Lava,
            11 => Material::StationaryLava,
            12 => Material::Sand,
            13 => Material::Gravel,
            14 => Material::GoldOre,
            15 => Material::IronOre,
            16 => Material::CoalOre,
            17 => Material::Log,
            18 => Material::Leaves,
            19 => Material::Sponge,
            20 => Material::Glass,
            21 => Material::LapisOre,
            22 => Material::LapisBlock,
            23 => Material::Dispenser,
            24 => Material::Sandstone,
            25 => Material::NoteBlock,
            26 => Material::BedBlock,
            27 => Material::PoweredRail,
            28 => Material::DetectorRail,
            29 => Material::PistonStickyBase,
            30 => Material::Web,
            31 => Material::LongGrass,
            32 => Material::DeadBush,
            33 => Material::PistonBase,
            34 => Material::PistonExtension,
            35 => Material::Wool,
            36 => Material::PistonMovingPiece,
            37 => Material::YellowFlower,
            38 => Material::RedRose,
            39 => Material::BrownMushroom,
            40 => Material::RedMushroom,
            41 => Material::GoldBlock,
            42 => Material::IronBlock,
            43 => Material::DoubleStep,
            44 => Material::Step,
            45 => Material::Brick,
            46 => Material::Tnt,
            47 => Material::Bookshelf,
            48 => Material::MossyCobblestone,
            49 => Material::Obsidian,
            50 => Material::Torch,
            51 => Material::Fire,
            52 => Material::MobSpawner,
            53 => Material::WoodStairs,
            54 => Material::Chest,
            55 => Material::RedstoneWire,
            56 => Material::DiamondOre,
            57 => Material::DiamondBlock,
            58 => Material::Workbench,
            59 => Material::Crops,
            60 => Material::Soil,
            61 => Material::Furnace,
            62 => Material::BurningFurnace,
            63 => Material::SignPost,
            64 => Material::WoodenDoor,
            65 => Material::Ladder,
            66 => Material::Rails,
            67 => Material::CobblestoneStairs,
            68 => Material::WallSign,
            69 => Material::Lever,
            70 => Material::StonePlate,
            71 => Material::IronDoorBlock,
            72 => Material::WoodPlate,
            73 => Material::RedstoneOre,
            74 => Material::GlowingRedstoneOre,
            75 => Material::RedstoneTorchOff,
            76 => Material::RedstoneTorchOn,
            77 => Material::StoneButton,
            78 => Material::Snow,
            79 => Material::Ice,
            80 => Material::SnowBlock,
            81 => Material::Cactus,
            82 => Material::Clay,
            83 => Material::SugarCaneBlock,
            84 => Material::Jukebox,
            85 => Material::Fence,
            86 => Material::Pumpkin,
            87 => Material::Netherrack,
            88 => Material::SoulSand,
            89 => Material::Glowstone,
            90 => Material::Portal,
            91 => Material::JackOLantern,
            92 => Material::CakeBlock,
            93 => Material::DiodeBlockOff,
            94 => Material::DiodeBlockOn,
            95 => Material::StainedGlass,
            96 => Material::TrapDoor,
            97 => Material::MonsterEggs,
            98 => Material::SmoothBrick,
            99 => Material::HugeMushroom1,
            100 => Material::HugeMushroom2,
            101 => Material::IronFence,
            102 => Material::ThinGlass,
            103 => Material::MelonBlock,
            104 => Material::PumpkinStem,
            105 => Material::MelonStem,
            106 => Material::Vine,
            107 => Material::FenceGate,
            108 => Material::BrickStairs,
            109 => Material::SmoothStairs,
            110 => Material::Mycel,
            111 => Material::WaterLily,
            112 => Material::NetherBrick,
            113 => Material::NetherFence,
            114 => Material::NetherBrickStairs,
            115 => Material::NetherWarts,
            116 => Material::EnchantmentTable,
            117 => Material::BrewingStand,
            118 => Material::Cauldron,
            119 => Material::EnderPortal,
            120 => Material::EnderPortalFrame,
            121 => Material::EnderStone,
            122 => Material::DragonEgg,
            123 => Material::RedstoneLampOff,
            124 => Material::RedstoneLampOn,
            125 => Material::WoodDoubleStep,
            126 => Material::WoodStep,
            127 => Material::Cocoa,
            128 => Material::SandstoneStairs,
            129 => Material::EmeraldOre,
            130 => Material::EnderChest,
            131 => Material::TripwireHook,
            132 => Material::Tripwire,
            133 => Material::EmeraldBlock,
            134 => Material::SpruceWoodStairs,
            135 => Material::BirchWoodStairs,
            136 => Material::JungleWoodStairs,
            137 => Material::Command,
            138 => Material::Beacon,
            139 => Material::CobbleWall,
            140 => Material::FlowerPot,
            141 => Material::Carrot,
            142 => Material::Potato,
            143 => Material::WoodButton,
            144 => Material::Skull,
            145 => Material::Anvil,
            146 => Material::TrappedChest,
            147 => Material::GoldPlate,
            148 => Material::IronPlate,
            149 => Material::RedstoneComparatorOff,
            150 => Material::RedstoneComparatorOn,
            151 => Material::DaylightDetector,
            152 => Material::RedstoneBlock,
            153 => Material::QuartzOre,
            154 => Material::Hopper,
            155 => Material::QuartzBlock,
            156 => Material::QuartzStairs,
            157 => Material::ActivatorRail,
            158 => Material::Dropper,
            159 => Material::StainedClay,
            160 => Material::StainedGlassPane,
            161 => Material::Leaves2,
            162 => Material::Log2,
            163 => Material::AcaciaStairs,
            164 => Material::DarkOakStairs,
            165 => Material::SlimeBlock,
            166 => Material::Barrier,
            167 => Material::IronTrapdoor,
            168 => Material::Prismarine,
            169 => Material::SeaLantern,
            170 => Material::HayBlock,
            171 => Material::Carpet,
            172 => Material::HardClay,
            173 => Material::CoalBlock,
            174 => Material::PackedIce,
            175 => Material::DoublePlant,
            176 => Material::StandingBanner,
            177 => Material::WallBanner,
            178 => Material::DaylightDetectorInverted,
            179 => Material::RedSandstone,
            180 => Material::RedSandstoneStairs,
            181 => Material::DoubleStoneSlab2,
            182 => Material::StoneSlab2,
            183 => Material::SpruceFenceGate,
            184 => Material::BirchFenceGate,
            185 => Material::JungleFenceGate,
            186 => Material::DarkOakFenceGate,
            187 => Material::AcaciaFenceGate,
            188 => Material::SpruceFence,
            189 => Material::BirchFence,
            190 => Material::JungleFence,
            191 => Material::DarkOakFence,
            192 => Material::AcaciaFence,
            193 => Material::SpruceDoor,
            194 => Material::BirchDoor,
            195 => Material::JungleDoor,
            196 => Material::AcaciaDoor,
            197 => Material::DarkOakDoor,
            256 => Material::IronSpade,
            257 => Material::IronPickaxe,
            258 => Material::IronAxe,
            259 => Material::FlintAndSteel,
            260 => Material::Apple,
            261 => Material::Bow,
            262 => Material::Arrow,
            263 => Material::Coal,
            264 => Material::Diamond,
            265 => Material::IronIngot,
            266 => Material::GoldIngot,
            267 => Material::IronSword,
            268 => Material::WoodSword,
            269 => Material::WoodSpade,
            270 => Material::WoodPickaxe,
            271 => Material::WoodAxe,
            272 => Material::StoneSword,
            273 => Material::StoneSpade,
            274 => Material::StonePickaxe,
            275 => Material::StoneAxe,
            276 => Material::DiamondSword,
            277 => Material::DiamondSpade,
            278 => Material::DiamondPickaxe,
            279 => Material::DiamondAxe,
            280 => Material::Stick,
            281 => Material::Bowl,
            282 => Material::MushroomSoup,
            283 => Material::GoldSword,
            284 => Material::GoldSpade,
            285 => Material::GoldPickaxe,
            286 => Material::GoldAxe,
            287 => Material::String,
            288 => Material::Feather,
            289 => Material::Sulphur,
            290 => Material::WoodHoe,
            291 => Material::StoneHoe,
            292 => Material::IronHoe,
            293 => Material::DiamondHoe,
            294 => Material::GoldHoe,
            295 => Material::Seeds,
            296 => Material::Wheat,
            297 => Material::Bread,
            298 => Material::LeatherHelmet,
            299 => Material::LeatherChestplate,
            300 => Material::LeatherLeggings,
            301 => Material::LeatherBoots,
            302 => Material::ChainmailHelmet,
            303 => Material::ChainmailChestplate,
            304 => Material::ChainmailLeggings,
            305 => Material::ChainmailBoots,
            306 => Material::IronHelmet,
            307 => Material::IronChestplate,
            308 => Material::IronLeggings,
            309 => Material::IronBoots,
            310 => Material::DiamondHelmet,
            311 => Material::DiamondChestplate,
            312 => Material::DiamondLeggings,
            313 => Material::DiamondBoots,
            314 => Material::GoldHelmet,
            315 => Material::GoldChestplate,
            316 => Material::GoldLeggings,
            317 => Material::GoldBoots,
            318 => Material::Flint,
            319 => Material::Pork,
            320 => Material::GrilledPork,
            321 => Material::Painting,
            322 => Material::GoldenApple,
            323 => Material::Sign,
            324 => Material::WoodDoor,
            325 => Material::Bucket,
            326 => Material::WaterBucket,
            327 => Material::LavaBucket,
            328 => Material::Minecart,
            329 => Material::Saddle,
            330 => Material::IronDoor,
            331 => Material::Redstone,
            332 => Material::SnowBall,
            333 => Material::Boat,
            334 => Material::Leather,
            335 => Material::MilkBucket,
            336 => Material::ClayBrick,
            337 => Material::ClayBall,
            338 => Material::SugarCane,
            339 => Material::Paper,
            340 => Material::Book,
            341 => Material::SlimeBall,
            342 => Material::StorageMinecart,
            343 => Material::PoweredMinecart,
            344 => Material::Egg,
            345 => Material::Compass,
            346 => Material::FishingRod,
            347 => Material::Watch,
            348 => Material::GlowstoneDust,
            349 => Material::RawFish,
            350 => Material::CookedFish,
            351 => Material::InkSack,
            352 => Material::Bone,
            353 => Material::Sugar,
            354 => Material::Cake,
            355 => Material::Bed,
            356 => Material::Diode,
            357 => Material::Cookie,
            358 => Material::Map,
            359 => Material::Shears,
            360 => Material::Melon,
            361 => Material::PumpkinSeeds,
            362 => Material::MelonSeeds,
            363 => Material::RawBeef,
            364 => Material::CookedBeef,
            365 => Material::RawChicken,
            366 => Material::CookedChicken,
            367 => Material::RottenFlesh,
            368 => Material::EnderPearl,
            369 => Material::BlazeRod,
            370 => Material::GhastTear,
            371 => Material::GoldNugget,
            372 => Material::NetherStalk,
            373 => Material::Potion,
            374 => Material::GlassBottle,
            375 => Material::SpiderEye,
            376 => Material::FermentedSpiderEye,
            377 => Material::BlazePowder,
            378 => Material::MagmaCream,
            379 => Material::BrewingStandItem,
            380 => Material::CauldronItem,
            381 => Material::EyeOfEnder,
            382 => Material::SpeckledMelon,
            383 => Material::MonsterEgg,
            384 => Material::ExpBottle,
            385 => Material::Fireball,
            386 => Material::BookAndQuill,
            387 => Material::WrittenBook,
            388 => Material::Emerald,
            389 => Material::ItemFrame,
            390 => Material::FlowerPotItem,
            391 => Material::CarrotItem,
            392 => Material::PotatoItem,
            393 => Material::BakedPotato,
            394 => Material::PoisonousPotato,
            395 => Material::EmptyMap,
            396 => Material::GoldenCarrot,
            397 => Material::SkullItem,
            398 => Material::CarrotStick,
            399 => Material::NetherStar,
            400 => Material::PumpkinPie,
            401 => Material::Firework,
            402 => Material::FireworkCharge,
            403 => Material::EnchantedBook,
            404 => Material::RedstoneComparator,
            405 => Material::NetherBrickItem,
            406 => Material::Quartz,
            407 => Material::ExplosiveMinecart,
            408 => Material::HopperMinecart,
            409 => Material::PrismarineShard,
            410 => Material::PrismarineCrystals,
            411 => Material::Rabbit,
            412 => Material::CookedRabbit,
            413 => Material::RabbitStew,
            414 => Material::RabbitFoot,
            415 => Material::RabbitHide,
            416 => Material::ArmorStand,
            417 => Material::IronBarding,
            418 => Material::GoldBarding,
            419 => Material::DiamondBarding,
            420 => Material::Leash,
            421 => Material::NameTag,
            422 => Material::CommandMinecart,
            423 => Material::Mutton,
            424 => Material::CookedMutton,
            425 => Material::Banner,
            427 => Material::SpruceDoorItem,
            428 => Material::BirchDoorItem,
            429 => Material::JungleDoorItem,
            430 => Material::AcaciaDoorItem,
            431 => Material::DarkOakDoorItem,
            2256 => Material::GoldRecord,
            2257 => Material::GreenRecord,
            2258 => Material::Record3,
            2259 => Material::Record4,
            2260 => Material::Record5,
            2261 => Material::Record6,
            2262 => Material::Record7,
            2263 => Material::Record8,
            2264 => Material::Record9,
            2265 => Material::Record10,
            2266 => Material::Record11,
            2267 => Material::Record12,
            _ => Material::Stone,
            // _ => unimplemented!("This Material does not exits id: {}", id),
        }
    }
}
