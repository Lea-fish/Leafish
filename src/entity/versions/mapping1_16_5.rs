use crate::entity::EntityType;
/*
These entities may be null (wrong id):
- SplashPotion
- Egg (here: ThrownEgg)
- FishingHook
- Lightning
- Weather
- Player
- ComplexPart
 */

pub fn to_id(entity_type: EntityType) -> i16 {
    match entity_type {
        EntityType::DroppedItem => 1,
        EntityType::ExperienceOrb => 2,
        EntityType::AreaEffectCloud => 3,
        EntityType::ElderGuardian => 4,
        EntityType::WitherSkeleton => 5,
        EntityType::Stray => 6,
        EntityType::Egg => 7,
        EntityType::LeashHitch => 8,
        EntityType::Painting => 9,
        EntityType::Arrow => 10,
        EntityType::Snowball => 11,
        EntityType::Fireball => 12,
        EntityType::SmallFireball => 13,
        EntityType::EnderPearl => 14,
        EntityType::EnderSignal => 15,
        EntityType::SplashPotion => 16,
        EntityType::ThrownExpBottle => 17,
        EntityType::ItemFrame => 18,
        EntityType::WitherSkull => 19,
        EntityType::PrimedTnt => 20,
        EntityType::FallingBlock => 21,
        EntityType::Firework => 22,
        EntityType::Husk => 23,
        EntityType::SpectralArrow => 24,
        EntityType::ShulkerBullet => 25,
        EntityType::DragonFireball => 26,
        EntityType::ZombieVillager => 27,
        EntityType::SkeletonHorse => 28,
        EntityType::ZombieHorse => 29,
        EntityType::ArmorStand => 30,
        EntityType::Donkey => 31,
        EntityType::Mule => 32,
        EntityType::EvokerFangs => 33,
        EntityType::Evoker => 34,
        EntityType::Vex => 35,
        EntityType::Vindicator => 36,
        EntityType::Illusioner => 37,
        EntityType::MinecartCommand => 40,
        EntityType::Boat => 41,
        EntityType::Minecart => 42,
        EntityType::MinecartChest => 43,
        EntityType::MinecartFurnace => 44,
        EntityType::MinecartTnt => 45,
        EntityType::MinecartHopper => 46,
        EntityType::MinecartMobSpawner => 47,
        EntityType::Creeper => 50,
        EntityType::Skeleton => 51,
        EntityType::Spider => 52,
        EntityType::Giant => 53,
        EntityType::Zombie => 54,
        EntityType::Slime => 55,
        EntityType::Ghast => 56,
        EntityType::ZombifiedPiglin => 57,
        EntityType::Enderman => 58,
        EntityType::CaveSpider => 59,
        EntityType::Silverfish => 60,
        EntityType::Blaze => 61,
        EntityType::MagmaCube => 62,
        EntityType::EnderDragon => 63,
        EntityType::Wither => 64,
        EntityType::Bat => 65,
        EntityType::Witch => 66,
        EntityType::Endermite => 67,
        EntityType::Guardian => 68,
        EntityType::Shulker => 69,
        EntityType::Pig => 90,
        EntityType::Sheep => 91,
        EntityType::Cow => 92,
        EntityType::Chicken => 93,
        EntityType::Squid => 94,
        EntityType::Wolf => 95,
        EntityType::MushroomCow => 96,
        EntityType::Snowman => 97,
        EntityType::Ocelot => 98,
        EntityType::IronGolem => 99,
        EntityType::Horse => 100,
        EntityType::Rabbit => 101,
        EntityType::PolarBear => 102,
        EntityType::Llama => 103,
        EntityType::LlamaSpit => 104,
        EntityType::Parrot => 105,
        EntityType::Villager => 120,
        EntityType::EnderCrystal => 200,
        EntityType::Turtle => -1,
        EntityType::Phantom => -1,
        EntityType::Trident => -1,
        EntityType::Cod => -1,
        EntityType::Salmon => -1,
        EntityType::Pufferfish => -1,
        EntityType::TropicalFish => -1,
        EntityType::Drowned => -1,
        EntityType::Dolphin => -1,
        EntityType::Cat => -1,
        EntityType::Panda => -1,
        EntityType::Pillager => -1,
        EntityType::Ravager => -1,
        EntityType::TraderLlama => -1,
        EntityType::WanderingTrader => -1,
        EntityType::Fox => -1,
        EntityType::Bee => -1,
        EntityType::Hoglin => -1,
        EntityType::Piglin => -1,
        EntityType::Strider => -1,
        EntityType::Zoglin => -1,
        EntityType::PiglinBrute => -1,
        EntityType::FishingHook => -1,
        EntityType::Lightning => -1,
        EntityType::Player => -1,
        EntityType::Unknown => -1,
        _ => -1,
    }
}

pub fn to_entity_type(type_id: i16) -> EntityType {
    match type_id {
        1 => EntityType::DroppedItem,
        2 => EntityType::ExperienceOrb,
        3 => EntityType::AreaEffectCloud,
        4 => EntityType::ElderGuardian,
        5 => EntityType::WitherSkeleton,
        6 => EntityType::Stray,
        7 => EntityType::Egg,
        8 => EntityType::LeashHitch,
        9 => EntityType::Painting,
        10 => EntityType::Arrow,
        11 => EntityType::Snowball,
        12 => EntityType::Fireball,
        13 => EntityType::SmallFireball,
        14 => EntityType::EnderPearl,
        15 => EntityType::EnderSignal,
        16 => EntityType::SplashPotion,
        17 => EntityType::ThrownExpBottle,
        18 => EntityType::ItemFrame,
        19 => EntityType::WitherSkull,
        20 => EntityType::PrimedTnt,
        21 => EntityType::FallingBlock,
        22 => EntityType::Firework,
        23 => EntityType::Husk,
        24 => EntityType::SpectralArrow,
        25 => EntityType::ShulkerBullet,
        26 => EntityType::DragonFireball,
        27 => EntityType::ZombieVillager,
        28 => EntityType::SkeletonHorse,
        29 => EntityType::ZombieHorse,
        30 => EntityType::ArmorStand,
        31 => EntityType::Donkey,
        32 => EntityType::Mule,
        33 => EntityType::EvokerFangs,
        34 => EntityType::Evoker,
        35 => EntityType::Vex,
        36 => EntityType::Vindicator,
        37 => EntityType::Illusioner,
        40 => EntityType::MinecartCommand,
        41 => EntityType::Boat,
        42 => EntityType::Minecart,
        43 => EntityType::MinecartChest,
        44 => EntityType::MinecartFurnace,
        45 => EntityType::MinecartTnt,
        46 => EntityType::MinecartHopper,
        47 => EntityType::MinecartMobSpawner,
        50 => EntityType::Creeper,
        51 => EntityType::Skeleton,
        52 => EntityType::Spider,
        53 => EntityType::Giant,
        54 => EntityType::Zombie,
        55 => EntityType::Slime,
        56 => EntityType::Ghast,
        57 => EntityType::ZombifiedPiglin,
        58 => EntityType::Enderman,
        59 => EntityType::CaveSpider,
        60 => EntityType::Silverfish,
        61 => EntityType::Blaze,
        62 => EntityType::MagmaCube,
        63 => EntityType::EnderDragon,
        64 => EntityType::Wither,
        65 => EntityType::Bat,
        66 => EntityType::Witch,
        67 => EntityType::Endermite,
        68 => EntityType::Guardian,
        69 => EntityType::Shulker,
        90 => EntityType::Pig,
        91 => EntityType::Sheep,
        92 => EntityType::Cow,
        93 => EntityType::Chicken,
        94 => EntityType::Squid,
        95 => EntityType::Wolf,
        96 => EntityType::MushroomCow,
        97 => EntityType::Snowman,
        98 => EntityType::Ocelot,
        99 => EntityType::IronGolem,
        100 => EntityType::Horse,
        101 => EntityType::Rabbit,
        102 => EntityType::PolarBear,
        103 => EntityType::Llama,
        104 => EntityType::LlamaSpit,
        105 => EntityType::Parrot,
        120 => EntityType::Villager,
        200 => EntityType::EnderCrystal,
        -1 => EntityType::Turtle,
        -1 => EntityType::Phantom,
        -1 => EntityType::Trident,
        -1 => EntityType::Cod,
        -1 => EntityType::Salmon,
        -1 => EntityType::Pufferfish,
        -1 => EntityType::TropicalFish,
        -1 => EntityType::Drowned,
        -1 => EntityType::Dolphin,
        -1 => EntityType::Cat,
        -1 => EntityType::Panda,
        -1 => EntityType::Pillager,
        -1 => EntityType::Ravager,
        -1 => EntityType::TraderLlama,
        -1 => EntityType::WanderingTrader,
        -1 => EntityType::Fox,
        -1 => EntityType::Bee,
        -1 => EntityType::Hoglin,
        -1 => EntityType::Piglin,
        -1 => EntityType::Strider,
        -1 => EntityType::Zoglin,
        -1 => EntityType::PiglinBrute,
        -1 => EntityType::FishingHook,
        -1 => EntityType::Lightning,
        -1 => EntityType::Player,
        -1 => EntityType::Unknown,
        _ => EntityType::Unknown,
    }
}