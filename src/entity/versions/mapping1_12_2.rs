#![allow(unreachable_patterns)]

use crate::entity::EntityType;

pub fn to_id(entity_type: EntityType) -> i16 {
    match entity_type {
        EntityType::AreaEffectCloud => 3,
        EntityType::ArmorStand => 30,
        EntityType::Arrow => 10,
        EntityType::Bat => 65,
        EntityType::Blaze => 61,
        EntityType::Boat => 41,
        EntityType::CaveSpider => 59,
        EntityType::Chicken => 93,
        EntityType::ComplexPart => -1,
        EntityType::Cow => 92,
        EntityType::Creeper => 50,
        EntityType::Donkey => 31,
        EntityType::DragonFireball => 26,
        EntityType::DroppedItem => 1,
        EntityType::Egg => 7,
        EntityType::ElderGuardian => 4,
        EntityType::Enderman => 58,
        EntityType::Endermite => 67,
        EntityType::EnderCrystal => 200,
        EntityType::EnderDragon => 63,
        EntityType::EnderPearl => 14,
        EntityType::EnderSignal => 15,
        EntityType::Evoker => 34,
        EntityType::EvokerFangs => 33,
        EntityType::ExperienceOrb => 2,
        EntityType::FallingBlock => 21,
        EntityType::Fireball => 12,
        EntityType::Firework => 22,
        EntityType::FishingHook => -1,
        EntityType::Ghast => 56,
        EntityType::Giant => 53,
        EntityType::Guardian => 68,
        EntityType::Horse => 100,
        EntityType::Husk => 23,
        EntityType::Illusioner => 37,
        EntityType::IronGolem => 99,
        EntityType::ItemFrame => 18,
        EntityType::LeashHitch => 8,
        EntityType::Lightning => -1,
        EntityType::LingeringPotion => -1,
        EntityType::Llama => 103,
        EntityType::LlamaSpit => 104,
        EntityType::MagmaCube => 62,
        EntityType::Minecart => 42,
        EntityType::MinecartChest => 43,
        EntityType::MinecartCommand => 40,
        EntityType::MinecartFurnace => 44,
        EntityType::MinecartHopper => 46,
        EntityType::MinecartMobSpawner => 47,
        EntityType::MinecartTnt => 45,
        EntityType::Mule => 32,
        EntityType::MushroomCow => 96,
        EntityType::Ocelot => 98,
        EntityType::Painting => 9,
        EntityType::Parrot => 105,
        EntityType::Pig => 90,
        EntityType::PigZombie => 57,
        EntityType::Player => -1,
        EntityType::PolarBear => 102,
        EntityType::PrimedTnt => 20,
        EntityType::Rabbit => 101,
        EntityType::Sheep => 91,
        EntityType::Shulker => 69,
        EntityType::ShulkerBullet => 25,
        EntityType::Silverfish => 60,
        EntityType::Skeleton => 51,
        EntityType::SkeletonHorse => 28,
        EntityType::Slime => 55,
        EntityType::SmallFireball => 13,
        EntityType::Snowball => 11,
        EntityType::Snowman => 97,
        EntityType::SpectralArrow => 24,
        EntityType::Spider => 52,
        EntityType::SplashPotion => 16,
        EntityType::Squid => 94,
        EntityType::Stray => 6,
        EntityType::ThrownExpBottle => 17,
        EntityType::TippedArrow => -1,
        EntityType::Vex => 35,
        EntityType::Villager => 120,
        EntityType::Vindicator => 36,
        EntityType::Weather => -1,
        EntityType::Witch => 66,
        EntityType::Wither => 64,
        EntityType::WitherSkeleton => 5,
        EntityType::WitherSkull => 19,
        EntityType::Wolf => 95,
        EntityType::Zombie => 54,
        EntityType::ZombieHorse => 29,
        EntityType::ZombieVillager => 27,
        EntityType::Unknown => -1,
        _ => -1,
    }
}

pub fn to_entity_type(type_id: i16) -> EntityType {
    match type_id {
        3 => EntityType::AreaEffectCloud,
        30 => EntityType::ArmorStand,
        10 => EntityType::Arrow,
        65 => EntityType::Bat,
        61 => EntityType::Blaze,
        41 => EntityType::Boat,
        59 => EntityType::CaveSpider,
        93 => EntityType::Chicken,
        -1 => EntityType::ComplexPart,
        92 => EntityType::Cow,
        50 => EntityType::Creeper,
        31 => EntityType::Donkey,
        26 => EntityType::DragonFireball,
        1 => EntityType::DroppedItem,
        7 => EntityType::Egg,
        4 => EntityType::ElderGuardian,
        58 => EntityType::Enderman,
        67 => EntityType::Endermite,
        200 => EntityType::EnderCrystal,
        63 => EntityType::EnderDragon,
        14 => EntityType::EnderPearl,
        15 => EntityType::EnderSignal,
        34 => EntityType::Evoker,
        33 => EntityType::EvokerFangs,
        2 => EntityType::ExperienceOrb,
        21 => EntityType::FallingBlock,
        12 => EntityType::Fireball,
        22 => EntityType::Firework,
        -1 => EntityType::FishingHook,
        56 => EntityType::Ghast,
        53 => EntityType::Giant,
        68 => EntityType::Guardian,
        100 => EntityType::Horse,
        23 => EntityType::Husk,
        37 => EntityType::Illusioner,
        99 => EntityType::IronGolem,
        18 => EntityType::ItemFrame,
        8 => EntityType::LeashHitch,
        -1 => EntityType::Lightning,
        -1 => EntityType::LingeringPotion,
        103 => EntityType::Llama,
        104 => EntityType::LlamaSpit,
        62 => EntityType::MagmaCube,
        42 => EntityType::Minecart,
        43 => EntityType::MinecartChest,
        40 => EntityType::MinecartCommand,
        44 => EntityType::MinecartFurnace,
        46 => EntityType::MinecartHopper,
        47 => EntityType::MinecartMobSpawner,
        45 => EntityType::MinecartTnt,
        32 => EntityType::Mule,
        96 => EntityType::MushroomCow,
        98 => EntityType::Ocelot,
        9 => EntityType::Painting,
        105 => EntityType::Parrot,
        90 => EntityType::Pig,
        57 => EntityType::PigZombie,
        -1 => EntityType::Player,
        102 => EntityType::PolarBear,
        20 => EntityType::PrimedTnt,
        101 => EntityType::Rabbit,
        91 => EntityType::Sheep,
        69 => EntityType::Shulker,
        25 => EntityType::ShulkerBullet,
        60 => EntityType::Silverfish,
        51 => EntityType::Skeleton,
        28 => EntityType::SkeletonHorse,
        55 => EntityType::Slime,
        13 => EntityType::SmallFireball,
        11 => EntityType::Snowball,
        97 => EntityType::Snowman,
        24 => EntityType::SpectralArrow,
        52 => EntityType::Spider,
        16 => EntityType::SplashPotion,
        94 => EntityType::Squid,
        6 => EntityType::Stray,
        17 => EntityType::ThrownExpBottle,
        -1 => EntityType::TippedArrow,
        35 => EntityType::Vex,
        120 => EntityType::Villager,
        36 => EntityType::Vindicator,
        -1 => EntityType::Weather,
        66 => EntityType::Witch,
        64 => EntityType::Wither,
        5 => EntityType::WitherSkeleton,
        19 => EntityType::WitherSkull,
        95 => EntityType::Wolf,
        54 => EntityType::Zombie,
        29 => EntityType::ZombieHorse,
        27 => EntityType::ZombieVillager,
        -1 => EntityType::Unknown,
        _ => EntityType::Unknown,
    }
}