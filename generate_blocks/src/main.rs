use convert_case::{Case, Casing};
use leafish_shared::Version;
use minecraft_data_rs::api::versions_by_minecraft_version;
use minecraft_data_rs::models::block_collision_shapes::{
    BlockCollisionShapes, CollisionShape, CollisionShapeIds,
};
use minecraft_data_rs::models::{block, item};
use minecraft_data_rs::Api;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

#[derive(Clone)]
struct BlockProp {
    name: String,
    state_type: block::StateType,
    values: Vec<String>,
}

impl BlockProp {
    fn new(name: &str, state_type: block::StateType, values: Vec<&str>) -> Self {
        let values = values.into_iter().map(|s| s.into()).collect();
        let state_type = match state_type {
            block::StateType::Enum if Self::is_int_enum(&values) => block::StateType::Int,
            s => s,
        };

        Self {
            name: name.into(),
            state_type,
            values,
        }
    }

    fn get_state_type(&self, block_name: &str) -> &'static str {
        match self.state_type {
            block::StateType::Bool => "bool",
            block::StateType::Int => "u8",
            block::StateType::Enum => self.get_enum_state_type(block_name).0,
        }
    }

    // Sometimes it seems like int values are marked as enums when their
    // first value isn't 0.
    fn is_int_enum(values: &Vec<String>) -> bool {
        for val in values {
            if u8::from_str(val).is_err() {
                return false;
            }
        }
        true
    }

    fn get_enum_state_type(&self, block_name: &str) -> (&'static str, usize) {
        let enum_types: HashMap<&str, (&str, usize)> = HashMap::from([
            ("axis", ("Axis", 4)),
            ("facing", ("Direction", 7)),
            ("instrument", ("NoteBlockInstrument", 16)),
            ("part", ("BedPart", 2)),
            ("hinge", ("Side", 2)),
            ("face", ("AttachedFace", 3)),
        ]);
        let directions: HashSet<&str> = HashSet::from(["east", "north", "south", "west"]);

        if let Some(type_) = enum_types.get(self.name.as_str()) {
            return *type_;
        }

        if self.name == "shape" {
            if block_name.ends_with("stairs") {
                return ("StairShape", 5);
            } else if block_name.ends_with("rail") {
                return ("RailShape", 10);
            }
        } else if self.name == "half" {
            if block_name == "tall_seagrass" {
                return ("TallSeagrassHalf", 2);
            } else if block_name.ends_with("_door") {
                return ("DoorHalf", 2);
            } else {
                // Assuming plant variants, stairs, or trapdoors.
                return ("BlockHalf", 5);
            }
        } else if self.name == "type" {
            if block_name.contains("piston") {
                return ("PistonType", 2);
            } else if block_name.contains("chest") {
                return ("ChestType", 3);
            } else if block_name.ends_with("_slab") {
                return ("BlockHalf", 5);
            }
        } else if directions.contains(self.name.as_str()) {
            if block_name == "redstone_wire" {
                return ("RedstoneSide", 3);
            } else if block_name.ends_with("_wall") {
                return ("WallSide", 3);
            }
        } else if self.name == "mode" {
            if block_name == "comparator" {
                return ("ComparatorMode", 2);
            } else if block_name == "structure_block" {
                return ("StructureBlockMode", 4);
            }
        } else if self.name == "leaves" {
            if block_name == "bamboo" {
                return ("BambooLeaves", 3);
            }
        } else if self.name == "attachment" {
            if block_name == "bell" {
                return ("BellAttachment", 4);
            }
        } else if self.name == "orientation" {
            if block_name == "jigsaw" {
                return ("JigsawOrientation", 12);
            }
        } else if self.name == "sculk_sensor_phase" {
            if block_name == "sculk_sensor" {
                return ("SculkSensorPhase", 3);
            }
        } else if self.name == "thickness" {
            if block_name == "pointed_dripstone" {
                return ("DripstoneThickness", 5);
            }
        } else if self.name == "vertical_direction" {
            if block_name == "pointed_dripstone" {
                return ("Direction", 7);
            }
        } else if self.name == "tilt" && block_name == "big_dripleaf" {
            return ("DripleafTilt", 4);
        }

        panic!(
            "Failed to find enum type for {} on block {}",
            self.name, block_name
        );
    }

    pub fn get_safe_name(&self) -> String {
        match self.name.as_str() {
            // "type" is a reserved word, so we need to use the rust field name
            // "type_" instead, while still using "type" while interactive with
            // minecraft-data.
            "type" => "type_".into(),
            s => s.into(),
        }
    }

    pub fn get_formatted_values(&self, block_name: &str) -> Vec<String> {
        match self.state_type {
            block::StateType::Enum => {
                let state_type = self.get_enum_state_type(block_name).0;
                self.values
                    .iter()
                    .map(|v| format!("{}::{}", state_type, v.to_case(Case::Pascal)))
                    .collect()
            }
            _ => self.values.clone(),
        }
    }

    pub fn are_values_exhaustive(&self, block_name: &str) -> bool {
        match self.state_type {
            block::StateType::Enum => {
                let total = self.get_enum_state_type(block_name).1;
                self.values.len() == total
            }
            block::StateType::Int => false,
            block::StateType::Bool => self.values.len() == 2,
        }
    }

    fn to_string(&self, block_name: &str) -> String {
        let values = self.get_formatted_values(block_name).join(", ");
        format!(
            "{}: {} = [{}]",
            self.get_safe_name(),
            self.get_state_type(block_name),
            values
        )
    }

    fn get_offset_str(&self, block_name: &str, multiplier: usize) -> String {
        match self.state_type {
            block::StateType::Int => {
                let offset: usize = self.values[0].parse().unwrap();

                // Ensure all values are continuous.
                for (i, value) in self.values.iter().enumerate() {
                    assert_eq!(*value, (i + offset).to_string())
                }

                let safe_name = self.get_safe_name();
                match (offset, multiplier) {
                    (0, 1) => format!("{} as usize", safe_name),
                    (_, 1) => format!("({} as usize - {})", safe_name, offset),
                    (0, _) => format!("{} as usize * {}", safe_name, multiplier),
                    (_, _) => format!("({} as usize - {}) * {}", safe_name, offset, multiplier),
                }
            }
            block::StateType::Bool => {
                format!(
                    "if {} {{ 0 }} else {{ {} }}",
                    self.get_safe_name(),
                    multiplier
                )
            }
            block::StateType::Enum => {
                let mut result = format!("match {} {{\n", self.get_safe_name());
                for (i, state) in self
                    .get_formatted_values(block_name)
                    .into_iter()
                    .enumerate()
                {
                    result += format!("    {} => {},\n", state, i * multiplier).as_str();
                }
                if !self.are_values_exhaustive(block_name) {
                    result += "    _ => unreachable!(),\n";
                }
                result += "}";
                result
            }
        }
    }
}

impl From<block::State> for BlockProp {
    fn from(state: block::State) -> Self {
        let values = match state.values {
            Some(values) => values,
            None => match state.state_type {
                block::StateType::Bool => {
                    vec!["true".into(), "false".into()]
                }
                block::StateType::Int => (0..state.num_values).map(|i| i.to_string()).collect(),
                block::StateType::Enum => {
                    panic!("No values available for enum state {}", state.name)
                }
            },
        };

        let state_type = match state.state_type {
            block::StateType::Enum if Self::is_int_enum(&values) => block::StateType::Int,
            s => s,
        };

        Self {
            name: state.name,
            state_type,
            values,
        }
    }
}

enum CollisionInfo {
    Single(CollisionShape),
    Multiple(Vec<CollisionShape>),
}

impl CollisionInfo {
    pub fn new(block: &block::Block, collision_shapes: &BlockCollisionShapes) -> Option<Self> {
        match &collision_shapes.blocks[&block.name] {
            CollisionShapeIds::Value(id) => {
                if *id == 1 {
                    None
                } else {
                    Some(Self::Single(collision_shapes.shapes[id].clone()))
                }
            }
            CollisionShapeIds::Array(ids) => Some(Self::Multiple(
                ids.iter()
                    .map(|id| collision_shapes.shapes[id].clone())
                    .collect(),
            )),
        }
    }
}

impl std::fmt::Display for CollisionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollisionInfo::Single(shapes) => {
                if shapes.is_empty() {
                    return write!(f, "{{ _ => [], }}");
                }

                writeln!(f, "{{")?;
                writeln!(f, "            _ => [")?;
                for shape in shapes {
                    write!(f, "                (")?;
                    write!(f, "({:?}, {:?}, {:?}), ", shape[0], shape[1], shape[2])?;
                    writeln!(f, "({:?}, {:?}, {:?})),", shape[3], shape[4], shape[5])?;
                }
                writeln!(f, "            ],")?;
                write!(f, "        }}")
            }
            CollisionInfo::Multiple(shapes_variants) => {
                writeln!(f, "{{")?;
                for (i, shapes) in shapes_variants.iter().enumerate() {
                    writeln!(f, "            {} => [", i)?;
                    for shape in shapes {
                        write!(f, "                (")?;
                        write!(f, "({:?}, {:?}, {:?}), ", shape[0], shape[1], shape[2])?;
                        writeln!(f, "({:?}, {:?}, {:?})),", shape[3], shape[4], shape[5])?;
                    }
                    writeln!(f, "            ],")?;
                }
                write!(f, "        }}")
            }
        }
    }
}

enum TintVariant {
    Constant(String),
    Conditional(&'static str, Vec<(&'static str, &'static str)>),
}

impl TintVariant {
    pub fn from_rgb(r: i32, g: i32, b: i32) -> TintVariant {
        let tint = format!("Color {{ r: {}, g: {}, b: {} }}", r, g, b);
        TintVariant::Constant(tint)
    }
}

impl std::fmt::Display for TintVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TintVariant::Constant(tint) => {
                write!(f, "        tint TintType::{tint},")
            }
            TintVariant::Conditional(var, tints) => {
                writeln!(f, "        tint match {var} {{")?;
                for (key, tint) in tints {
                    writeln!(f, "            {key} => TintType::{tint},")?;
                }
                writeln!(f, "            _ => unreachable!(),")?;
                write!(f, "        }},")
            }
        }
    }
}

enum ModelVariant {
    Single(Vec<BlockProp>),
    Multipart(Vec<BlockProp>),
}

impl std::fmt::Display for ModelVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelVariant::Single(props) => {
                let fmt = props
                    .iter()
                    .map(|prop| format!("{}={{}}", prop.name))
                    .collect::<Vec<String>>()
                    .join(",");

                let vars = props
                    .iter()
                    .map(|prop| match prop.state_type {
                        block::StateType::Enum => format!("{}.as_string()", prop.get_safe_name()),
                        _ => prop.get_safe_name(),
                    })
                    .collect::<Vec<String>>()
                    .join(", ");

                write!(f, "variant format!(\"{fmt}\", {vars})")
            }
            ModelVariant::Multipart(props) => {
                writeln!(f, "multipart (key, val) => match key {{")?;
                for prop in props {
                    let var = match prop.state_type {
                        block::StateType::Enum => format!("{}.as_string()", prop.get_safe_name()),
                        _ => format!("&{}.to_string()", prop.get_safe_name()),
                    };
                    writeln!(f, "            \"{}\" => val.contains({var}),", prop.name)?;
                }
                writeln!(f, "            _ => false,")?;
                write!(f, "        }}")
            }
        }
    }
}

#[derive(Clone)]
struct BlockStateInfo {
    name: String,
    props: Vec<BlockProp>,
    min_state_id: usize,
    max_state_id: usize,
}

impl BlockStateInfo {
    fn new(block: block::Block) -> Self {
        Self {
            name: block.name,
            props: match block.states {
                Some(states) => states.into_iter().map(|s| s.into()).collect(),
                None => vec![],
            },
            min_state_id: block.min_state_id.unwrap() as usize,
            max_state_id: block.max_state_id.unwrap() as usize,
        }
    }

    fn get_states(&self) -> Vec<String> {
        Self::enumerate_props(&self.name, &self.props)
            .iter()
            .map(|vars| vars.join(", "))
            .collect()
    }

    fn enumerate_props(block_name: &str, props: &[BlockProp]) -> Vec<Vec<String>> {
        if props.is_empty() {
            return vec![vec![]];
        }

        let mut result = vec![];
        for value in &props[0].get_formatted_values(block_name) {
            for previous_state in Self::enumerate_props(block_name, &props[1..]) {
                let current = vec![format!("{}: {}", props[0].get_safe_name(), value)]
                    .into_iter()
                    .chain(previous_state.iter().map(|s| s.into()))
                    .collect();
                result.push(current);
            }
        }
        result
    }

    fn get_offset_str(&self) -> Option<String> {
        if self.props.is_empty() {
            return None;
        }

        let mut result = vec![];
        let mut multiplier = 1;
        for prop in self.props.iter().rev() {
            result.push(prop.get_offset_str(self.name.as_str(), multiplier));
            multiplier *= prop.values.len();
        }

        if result.len() == 1 {
            Some(result[0].replace('\n', "\n        "))
        } else {
            let result = result
                .into_iter()
                .map(|s| s.replace('\n', "\n            "))
                .collect::<Vec<String>>()
                .join(" +\n            ");
            Some(format!("(\n            {}\n        )", result))
        }
    }

    fn upgrade_from(mut self, version: Version) -> Vec<Self> {
        let mut result = vec![];

        if version < Version::V1_13_2 {
            // 1.13.2 Added waterlogging to coral and conduits. Before this
            // they could only be placed in water.
            if self.name.ends_with("_coral") || self.name == "conduit" {
                self.add_dummy_state("waterlogged", block::StateType::Bool, "true");
            }

            // 1.13.2 Added the unstable flag to TNT
            if self.name == "tnt" {
                self.add_dummy_state("unstable", block::StateType::Bool, "false");
            }
        }

        if version < Version::V1_14 {
            // 1.14 Added multiple types of signs
            if self.name == "sign" {
                self.name = "oak_sign".into();
            }

            if self.name == "wall_sign" {
                self.name = "oak_wall_sign".into();
            }
        }

        if version < Version::V1_15 {
            // 1.15 Added a powered state to bells
            if self.name == "bell" {
                self.add_dummy_state("powered", block::StateType::Bool, "false");
            }
        }

        if version < Version::V1_16 {
            // 1.16 Added switched from a bool on each side to high and low states
            // on each side.
            if self.name.ends_with("_wall") {
                for prop in &mut self.props {
                    if ["east", "west", "north", "south"].contains(&prop.name.as_str()) {
                        prop.state_type = block::StateType::Enum;
                        prop.values = vec!["low".into(), "none".into()];
                    }
                }
            }

            // 1.16 Added orientation to a jigsaw block when facing up or down.
            if self.name == "jigsaw" {
                self.props.retain(|prop| prop.name != "facing");
                self.props.push(BlockProp::new(
                    "orientation",
                    block::StateType::Enum,
                    vec![
                        "north_up",
                        "east_up",
                        "south_up",
                        "west_up",
                        "up_east",
                        "down_east",
                    ],
                ));
            }
        }

        if version < Version::V1_16_2 {
            // 1.16.2 Added an axis for chains, rather than just vertical
            if self.name == "chain" {
                self.add_dummy_state("axis", block::StateType::Enum, "y");
            }

            // 1.16.2 Added waterlogging to lanterns
            if self.name == "lantern" || self.name == "soul_lantern" {
                self.add_dummy_state("waterlogged", block::StateType::Bool, "false");
            }
        }

        if version < Version::V1_17 {
            // 1.17 Renamed grass_path to dirt_path
            if self.name == "grass_path" {
                self.name = "dirt_path".into();
            }

            // 1.17 Added waterlogging to rails
            if self.name.ends_with("rail") {
                self.add_dummy_state("waterlogged", block::StateType::Bool, "false");
            }

            // 1.17 Allows cauldrons to contain lava and snow as well as water, so
            // the cauldron block has been replaced with "cauldron" if empty,
            // and "water_cauldron" if it contains water.
            if self.name == "cauldron" {
                let mut empty_cauldron = self.clone();
                empty_cauldron.props.clear();
                empty_cauldron.max_state_id = empty_cauldron.min_state_id;
                result.append(&mut empty_cauldron.upgrade_from(Version::V1_17));

                self.name = "water_cauldron".into();
                self.min_state_id += 1;
                self.props[0].values = vec!["1".into(), "2".into(), "3".into()];
            }
        }

        if version < Version::V1_19 {
            // 1.19 Added waterlogging to leaves
            if self.name.ends_with("_leaves") {
                self.add_dummy_state("waterlogged", block::StateType::Bool, "false");
            }
        }

        result.push(self);
        result
    }

    fn add_dummy_state(&mut self, name: &str, state_type: block::StateType, value: &str) {
        self.props
            .push(BlockProp::new(name, state_type, vec![value]));
    }

    fn get_variant_name(&self) -> String {
        self.name.to_case(Case::Pascal)
    }
}

impl std::fmt::Display for BlockStateInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for state in self.get_states() {
            writeln!(f, "    Block::{} {{ {state} }},", self.get_variant_name())?;
        }
        Ok(())
    }
}

struct BlockMetaInfo {
    block_info: BlockStateInfo,
    material: String,
    model: (String, String),
    model_variant: Option<ModelVariant>,
    tint: TintVariant,
    collision: Option<CollisionInfo>,
    hardness: Option<f32>,
    harvest_tools: Vec<String>,
    best_tools: Vec<String>,
    is_waterlogged: &'static str,
}

impl BlockMetaInfo {
    fn new(
        block: &block::Block,
        collision: Option<CollisionInfo>,
        blockstate: Value,
        items: &HashMap<u32, item::Item>,
    ) -> Self {
        let block_info = BlockStateInfo::new(block.clone());
        let model_variant = Self::get_model_variant(&block_info.props, blockstate);
        let hardness = match block.hardness {
            Some(hardness) if hardness >= 0.0 => block.hardness,
            _ => None,
        };

        let harvest_tools = block
            .harvest_tools
            .as_ref()
            .map(|tools| {
                let mut tools: Vec<&u32> = tools.keys().collect();
                tools.sort();
                tools
                    .into_iter()
                    .map(|id| &items[id])
                    .map(|tool| match tool.name.split_once('_') {
                        Some((material, kind)) => {
                            let kind = kind.to_case(Case::Pascal);
                            let material = material.to_case(Case::Pascal);
                            format!("Tool::{}(ToolMaterial::{})", kind, material)
                        }
                        None if tool.name == "shears" => "Tool::Shears".into(),
                        _ => panic!("Unexpected harvest tool {}", tool.name),
                    })
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        let material = block.material.clone().unwrap_or_else(|| "default".into());
        let best_tools = material
            .split(';')
            .flat_map(|m| match m {
                "default" => vec![],
                "plant" => vec![],
                "vine_or_glow_lichen" => vec![],
                "wool" => vec![],
                "mineable/pickaxe" => vec!["Tool::Pickaxe(_)"],
                "mineable/axe" => vec!["Tool::Axe(_)"],
                "mineable/shovel" => vec!["Tool::Shovel(_)"],
                "mineable/hoe" => vec!["Tool::Hoe(_)"],
                "coweb" => vec!["Tool::Sword(_)", "Tool::Shears"],
                "gourd" | "leaves" => vec!["Tool::Shears"],
                _ => panic!("Invalid material {}", m),
            })
            .map(|s| s.into())
            .collect();

        Self {
            block_info,
            material: Self::get_material(block, &collision),
            model: ("minecraft".into(), block.name.clone()),
            model_variant,
            tint: Self::get_tint(block),
            collision,
            hardness,
            harvest_tools,
            best_tools,
            is_waterlogged: Self::get_is_waterlogged(block),
        }
    }

    fn get_material(block: &block::Block, collision: &Option<CollisionInfo>) -> String {
        const AIR_MATERIAL: &str = "Material {
            renderable: false,
            should_cull_against: false,
            never_cull: false,
            force_shade: false,
            transparent: false,
            absorbed_light: 0,
            emitted_light: 0,
            collidable: false,
        }";

        if block.name == "air" {
            return AIR_MATERIAL.into();
        }

        let empty = matches!(block.bounding_box, block::BoundingBox::Empty);

        let mut renderable = true;
        let mut should_cull_against = !empty;
        let mut never_cull = false;
        let mut force_shade = false;
        let mut transparent = false;
        let mut absorbed_light = block.filter_light;
        let emitted_light = block.emit_light;
        let collidable = !empty;

        if block.name.contains("sign") {
            renderable = false;
            should_cull_against = false;
        }

        if block.name == "barrier" || block.name == "piston_head" {
            should_cull_against = false;
        }

        if block.name.contains("leaves") {
            never_cull = true;
            force_shade = true;
            transparent = false;
        }

        if block.transparent {
            absorbed_light = 1;
            should_cull_against = false;
        }

        if collision.is_some() {
            should_cull_against = false;
        }

        format!(
            "Material {{
            renderable: {renderable},
            should_cull_against: {should_cull_against},
            never_cull: {never_cull},
            force_shade: {force_shade},
            transparent: {transparent},
            absorbed_light: {absorbed_light},
            emitted_light: {emitted_light},
            collidable: {collidable},
        }}"
        )
    }

    fn get_tint(block: &block::Block) -> TintVariant {
        match block.name.as_str() {
            // Constants
            "attached_melon_stem" => TintVariant::from_rgb(224, 199, 28),
            "attached_pumpkin_stem" => TintVariant::from_rgb(224, 199, 28),
            "birch_leaves" => TintVariant::from_rgb(128, 167, 85),
            "lily_pad" => TintVariant::from_rgb(32, 128, 48),
            "melon_stem" => TintVariant::from_rgb(0, 255, 0),
            "pumpkin_stem" => TintVariant::from_rgb(0, 255, 0),
            "spruce_leaves" => TintVariant::from_rgb(97, 153, 97),

            // Grass
            // https://minecraft.fandom.com/wiki/Color#Grass
            "grass" => TintVariant::Constant("Grass".into()),
            "grass_block" => TintVariant::Constant("Grass".into()),
            "tall_grass" => TintVariant::Constant("Grass".into()),
            "fern" => TintVariant::Constant("Grass".into()),
            "large_fern" => TintVariant::Constant("Grass".into()),
            "potted_fern" => TintVariant::Constant("Grass".into()),
            "sugar_cane" => TintVariant::Constant("Grass".into()),

            // Foliage
            // https://minecraft.fandom.com/wiki/Color#Foliage
            "oak_leaves" => TintVariant::Constant("Foliage".into()),
            "jungle_leaves" => TintVariant::Constant("Foliage".into()),
            "acacia_leaves" => TintVariant::Constant("Foliage".into()),
            "dark_oak_leaves" => TintVariant::Constant("Foliage".into()),
            "vine" => TintVariant::Constant("Foliage".into()),

            // Water
            "water" => TintVariant::Constant("Water".into()),

            // Redstone
            "redstone_wire" => TintVariant::Conditional(
                "power",
                vec![
                    ("0", "Color { r: 76, g: 0, b: 0 }"),
                    ("1", "Color { r: 112, g: 0, b: 0 }"),
                    ("2", "Color { r: 122, g: 0, b: 0 }"),
                    ("3", "Color { r: 132, g: 0, b: 0 }"),
                    ("4", "Color { r: 142, g: 0, b: 0 }"),
                    ("5", "Color { r: 153, g: 0, b: 0 }"),
                    ("6", "Color { r: 163, g: 0, b: 0 }"),
                    ("7", "Color { r: 173, g: 0, b: 0 }"),
                    ("8", "Color { r: 183, g: 0, b: 0 }"),
                    ("9", "Color { r: 193, g: 0, b: 0 }"),
                    ("10", "Color { r: 204, g: 0, b: 0 }"),
                    ("11", "Color { r: 214, g: 0, b: 0 }"),
                    ("12", "Color { r: 224, g: 0, b: 0 }"),
                    ("13", "Color { r: 234, g: 6, b: 0 }"),
                    ("14", "Color { r: 244, g: 27, b: 0 }"),
                    ("15", "Color { r: 255, g: 50, b: 0 }"),
                ],
            ),

            _ => TintVariant::Constant("Default".into()),
        }
    }

    fn get_is_waterlogged(block: &block::Block) -> &'static str {
        // These blocks can only exist in water
        match block.name.as_str() {
            "tall_seagrass" => return "true",
            "seagrass" => return "true",
            "kelp" => return "true",
            "kelp_plant" => return "true",
            _ => {}
        };

        // These block only have a waterlogged state
        if let Some(states) = &block.states {
            for state in states {
                if state.name == "waterlogged" {
                    return "waterlogged";
                }
            }
        }

        "false"
    }

    pub fn get_model_variant(props: &[BlockProp], blockstate: Value) -> Option<ModelVariant> {
        if let Some(serde_json::Value::Object(variants)) = blockstate.get("variants") {
            let variants: Vec<&String> = variants.keys().filter(|v| !v.is_empty()).collect();
            if let Some(keys) = variants.get(0) {
                let props: HashMap<&str, &BlockProp> = props
                    .iter()
                    .map(|prop| (prop.name.as_str(), prop))
                    .collect();
                let props = keys
                    .split(',')
                    .map(|pattern| pattern.split('=').next().unwrap())
                    .map(|name| props[name].clone())
                    .collect();
                Some(ModelVariant::Single(props))
            } else {
                None
            }
        } else if !props.is_empty() {
            Some(ModelVariant::Multipart(props.into()))
        } else {
            None
        }
    }

    pub fn get_update_state(&self) -> Option<String> {
        let name = self.block_info.get_variant_name();
        match name.as_str() {
            "GrassBlock" | "Mycelium" => {
                Some(format!("{} {{ snowy: is_snowy(world, pos) }}", name))
            }
            "Fire" => {
                Some("update_fire_state(world, pos, age)".into())
            }
            "OakStairs" | "CobblestoneStairs" | "BrickStairs" | "StoneBrickStairs" |
            "NetherBrickStairs" | "SandstoneStairs" | "SpruceStairs" | "BirchStairs" |
            "JungleStairs" | "QuartzStairs" | "AcaciaStairs" | "DarkOakStairs" |
            "RedSandstoneStairs" | "PurpurStairs" => {
                Some(format!("{} {{
                    facing,
                    half,
                    shape: update_stair_shape(world, pos, facing),
                    waterlogged,
                }}", name))
            }
            "RedstoneWire" => {
                Some("update_redstone_state(world, pos, power)".into())
            }
            "OakDoor" | "IronDoor" | "SpruceDoor" | "BirchDoor" | "JungleDoor" |
            "AcaciaDoor" | "DarkOakDoor" => {
                Some(format!("{{
                    let (facing, hinge, open, powered) = update_door_state(world, pos, half, facing, hinge, open, powered);
                    {} {{ facing, half, hinge, open, powered }}
                }}", name))
            }
            "OakFence" | "SpruceFence" | "BirchFence" | "JungleFence" |
            "DarkOakFence" | "AcaciaFence" => {
                Some(format!("{{
                    let (north, south, west, east) = can_connect_sides(world, pos, &can_connect_fence);
                    {} {{ north, south, west, east, waterlogged }}
                }}", name))
            }
            "Repeater" => {
                Some("Repeater {
                    delay,
                    facing,
                    locked: update_repeater_state(world, pos, facing),
                    powered,
                }".into())
            }
            "IronBars" => {
                Some("{
                    let f = |block| matches!(block, IronBars { .. });
                    let (mut north, mut south, mut west, mut east) = can_connect_sides(world, pos, &f);
                    if !north && !south && !west && !east {{
                        (north, south, west, east) = (true, true, true, true);
                    }}
                    IronBars { north, south, west, east, waterlogged }
                }".into())
            }
            "GlassPane" | "WhiteStainedGlassPane" | "OrangeStainedGlassPane" |
            "MagentaStainedGlassPane" | "LightBlueStainedGlassPane" |
            "YellowStainedGlassPane" | "LimeStainedGlassPane" |
            "PinkStainedGlassPane" | "GrayStainedGlassPane" |
            "LightGrayStainedGlassPane" | "CyanStainedGlassPane" |
            "PurpleStainedGlassPane" | "BlueStainedGlassPane" |
            "BrownStainedGlassPane" | "GreenStainedGlassPane" |
            "RedStainedGlassPane" | "BlackStainedGlassPane" => {
                Some(format!("{{
                    let (mut north, mut south, mut west, mut east) = can_connect_sides(world, pos, &can_connect_glasspane);
                    if !north && !south && !west && !east {{
                        (north, south, west, east) = (true, true, true, true);
                    }}
                    {} {{ north, south, west, east, waterlogged }}
                }}", name))
            }
            "AttachedPumpkinStem" => {
                Some("{
                    let facing = match (
                        world.get_block(pos.shift(Direction::North)),
                        world.get_block(pos.shift(Direction::East)),
                        world.get_block(pos.shift(Direction::South)),
                        world.get_block(pos.shift(Direction::West)),
                    ) {
                        (CarvedPumpkin { .. }, _, _, _) => Direction::North,
                        (_, CarvedPumpkin { .. }, _, _) => Direction::East,
                        (_, _, CarvedPumpkin { .. }, _) => Direction::South,
                        (_, _, _, CarvedPumpkin { .. }) => Direction::West,
                        _ => return PumpkinStem { age: 7 }
                    };

                    AttachedPumpkinStem { facing }
                }".into())
            }
            "PumpkinStem" => {
                Some("{
                    let facing = match (
                        world.get_block(pos.shift(Direction::North)),
                        world.get_block(pos.shift(Direction::East)),
                        world.get_block(pos.shift(Direction::South)),
                        world.get_block(pos.shift(Direction::West)),
                    ) {
                        (CarvedPumpkin { .. }, _, _, _) => Direction::North,
                        (_, CarvedPumpkin { .. }, _, _) => Direction::East,
                        (_, _, CarvedPumpkin { .. }, _) => Direction::South,
                        (_, _, _, CarvedPumpkin { .. }) => Direction::West,
                        _ => return PumpkinStem { age }
                    };

                    AttachedPumpkinStem { facing }
                }".into())
            }
            "AttachedMelonStem" => {
                Some("{
                    let facing = match (
                        world.get_block(pos.shift(Direction::North)),
                        world.get_block(pos.shift(Direction::East)),
                        world.get_block(pos.shift(Direction::South)),
                        world.get_block(pos.shift(Direction::West)),
                    ) {
                        (Melon { .. }, _, _, _) => Direction::North,
                        (_, Melon { .. }, _, _) => Direction::East,
                        (_, _, Melon { .. }, _) => Direction::South,
                        (_, _, _, Melon { .. }) => Direction::West,
                        _ => return MelonStem { age: 7 }
                    };

                    AttachedMelonStem { facing }
                }".into())
            }
            "MelonStem" => {
                Some("{
                    let facing = match (
                        world.get_block(pos.shift(Direction::North)),
                        world.get_block(pos.shift(Direction::East)),
                        world.get_block(pos.shift(Direction::South)),
                        world.get_block(pos.shift(Direction::West)),
                    ) {
                        (Melon { .. }, _, _, _) => Direction::North,
                        (_, Melon { .. }, _, _) => Direction::East,
                        (_, _, Melon { .. }, _) => Direction::South,
                        (_, _, _, Melon { .. }) => Direction::West,
                        _ => return MelonStem { age }
                    };

                    AttachedMelonStem { facing }
                }".into())
            }
            "Vine" => {
                Some("{
                    let mat = world.get_block(pos.shift(Direction::Up)).get_material();
                    let up = mat.renderable && (mat.should_cull_against || mat.never_cull /* Because leaves */);
                    Vine { up, south, west, north, east }
                }".into())
            }
            "OakFenceGate" | "SpruceFenceGate" | "BirchFenceGate" | "JungleFenceGate" |
            "DarkOakFenceGate" | "AcaciaFenceGate" => {
                Some(format!("{} {{
                    facing,
                    in_wall: fence_gate_update_state(world, pos, facing),
                    open,
                    powered,
                }}", name))
            }
            "Tripwire" => {
                Some("{
                    let f = |dir| {
                        match world.get_block(pos.shift(dir)) {
                            TripwireHook { facing, .. } => facing.opposite() == dir,
                            Tripwire { .. } => true,
                            _ => false,
                        }
                    };

                    Tripwire {
                        powered,
                        attached,
                        disarmed,
                        east: f(Direction::East),
                        north: f(Direction::North),
                        south: f(Direction::South),
                        west: f(Direction::West),
                    }
                }".into())
            }
            "NetherBrickFence" => {
                Some("{
                    let f = |block| matches!(block, NetherBrickFence { .. } |
                        OakFenceGate { .. } |
                        SpruceFenceGate { .. } |
                        BirchFenceGate { .. } |
                        JungleFenceGate { .. } |
                        DarkOakFenceGate { .. } |
                        AcaciaFenceGate { .. });

                    let (north, south, west, east) = can_connect_sides(world, pos, &f);
                    NetherBrickFence { north, south, west, east, waterlogged }
                }".into())
            }
            "CobblestoneWall" | "MossyCobblestoneWall" => {
                Some(format!("{{
                    let (up, north, south, west, east) = update_wall_state(world, pos);
                    {} {{ up, north, south, west, east, waterlogged }}
                }}", name))
            }
            "Sunflower" | "Lilac" | "TallGrass" | "LargeFern" | "RoseBush" | "Peony" => {
                Some("update_double_plant_state(world, pos, half)".into())
            }
            "ChorusPlant" => {
                Some("ChorusPlant {
                    up: matches!(world.get_block(pos.shift(Direction::Up)), ChorusPlant { .. } | ChorusFlower { .. }),
                    down: matches!(world.get_block(pos.shift(Direction::Down)), ChorusPlant { .. } | ChorusFlower { .. } | EndStone { .. }),
                    north: matches!(world.get_block(pos.shift(Direction::North)), ChorusPlant { .. } | ChorusFlower { .. }),
                    south: matches!(world.get_block(pos.shift(Direction::South)), ChorusPlant { .. } | ChorusFlower { .. }),
                    west: matches!(world.get_block(pos.shift(Direction::West)), ChorusPlant { .. } | ChorusFlower { .. }),
                    east: matches!(world.get_block(pos.shift(Direction::East)), ChorusPlant { .. } | ChorusFlower { .. }),
                }".into())
            }
            _ => None,
        }
    }
}

impl std::fmt::Display for BlockMetaInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "    {} {{", self.block_info.get_variant_name())?;

        // props
        write!(f, "        props {{")?;
        if !self.block_info.props.is_empty() {
            writeln!(f)?;
            for prop in &self.block_info.props {
                writeln!(f, "            {},", prop.to_string(&self.block_info.name))?;
            }
            write!(f, "        ")?;
        }
        writeln!(f, "}},")?;

        // offset
        if let Some(offset_str) = self.block_info.get_offset_str() {
            writeln!(f, "        offset {},", offset_str)?;
        }

        // material
        writeln!(f, "        material material::{},", self.material)?;

        // model
        writeln!(
            f,
            "        model {{ (\"{}\", \"{}\") }},",
            self.model.0, self.model.1
        )?;

        // variant or multipart
        if let Some(variant) = &self.model_variant {
            writeln!(f, "        {},", variant)?;
        }

        // tint
        match &self.tint {
            TintVariant::Constant(tint) if tint == "Default" => {}
            tint => writeln!(f, "{}", tint)?,
        }

        // collision
        if let Some(collision) = &self.collision {
            writeln!(f, "        collision {collision},")?;
        }

        // update_state
        if let Some(update_state) = self.get_update_state() {
            let update_state = update_state.replace("\n        ", "\n");
            writeln!(f, "        update_state (world, pos) => {update_state},")?;
        }

        // hardness
        if let Some(hardness) = &self.hardness {
            writeln!(f, "        hardness {hardness:?},")?;
        }

        // harvest_tools
        if !self.harvest_tools.is_empty() {
            writeln!(f, "        harvest_tools [")?;
            for tool in &self.harvest_tools {
                writeln!(f, "            {},", tool)?;
            }
            writeln!(f, "        ],")?;
        }

        // best_tools
        if !self.best_tools.is_empty() {
            write!(f, "        best_tools [ ")?;
            for tool in &self.best_tools {
                write!(f, "{}, ", tool)?;
            }
            writeln!(f, "],")?;
        }

        // is_waterlogged
        if self.is_waterlogged != "false" {
            writeln!(f, "        is_waterlogged {},", self.is_waterlogged)?;
        }

        write!(f, "    }}")
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        let usage = format!("Usage: {} <resources-dir> <output-dir>\n", args[0])
            + &format!(
                "Example: {} ~/.local/share/leafish/resources-1.19.2 blocks/src/",
                args[0]
            );
        println!("{}", usage);
        return Ok(());
    }

    let resources_dir = Path::new(&args[1]);
    let blockstate_dir = resources_dir.join("assets/minecraft/blockstates");
    let output_dir = Path::new(&args[2]);

    let versions = versions_by_minecraft_version().unwrap();
    let api = Api::new(versions["1.19"].clone());
    let target_blocks = api.blocks.blocks_array().unwrap();
    let target_items = api.items.items().unwrap();
    let collision_shapes = api.blocks.block_collision_shapes().unwrap();

    // Write data to the blocks.rs file
    {
        let mut blocks_file = File::create(output_dir.join("blocks.rs")).unwrap();

        writeln!(blocks_file, "use crate::*;\n")?;
        writeln!(blocks_file, "define_blocks! {{")?;

        for block in &target_blocks {
            let collision = CollisionInfo::new(block, &collision_shapes);
            let blockstate_path = blockstate_dir.join(format!("{}.json", block.name));
            let blockstate = std::fs::read_to_string(blockstate_path).unwrap();
            let blockstate = serde_json::from_str(blockstate.as_str()).unwrap();
            writeln!(
                blocks_file,
                "{}",
                BlockMetaInfo::new(block, collision, blockstate, &target_items)
            )?;
        }

        writeln!(
            blocks_file,
            "{}\n}}",
            BlockMetaInfo {
                block_info: BlockStateInfo {
                    name: "missing".into(),
                    props: vec![],
                    min_state_id: usize::MAX,
                    max_state_id: usize::MAX,
                },
                material: "SOLID".into(),
                model: ("leafish".into(), "missing_block".into()),
                model_variant: None,
                tint: TintVariant::Constant("Default".into()),
                collision: None,
                hardness: None,
                harvest_tools: vec![],
                best_tools: vec![],
                is_waterlogged: "false",
            }
        )?;
    }

    let supported_versions = [
        ("1.13", Version::V1_13),
        ("1.13.2", Version::V1_13_2),
        ("1.14.4", Version::V1_14),
        ("1.15.2", Version::V1_15),
        ("1.16.1", Version::V1_16),
        ("1.16.5", Version::V1_16_2),
        ("1.17.1", Version::V1_17),
        ("1.18.2", Version::V1_18),
        ("1.19", Version::V1_19),
    ];

    // Write the mapping code for each version
    for (version_str, version) in supported_versions {
        let api = Api::new(versions[version_str].clone());
        let blocks = api.blocks.blocks_array().unwrap();
        let blocks: Vec<BlockStateInfo> = blocks
            .into_iter()
            .flat_map(|block| BlockStateInfo::new(block).upgrade_from(version))
            .collect();
        let mut current_state_id = 0;

        let mapping_path =
            output_dir.join(format!("versions/v{}.rs", version_str.replace('.', "_")));
        let mut mapping_file = File::create(mapping_path).unwrap();

        writeln!(
            mapping_file,
            "// This file was autogenerated by `generate_blocks`."
        )?;
        writeln!(mapping_file, "// Do not modify this file by hand.\n")?;
        writeln!(mapping_file, "use crate::*;")?;
        writeln!(mapping_file, "use crate::blocks::Block;\n")?;
        writeln!(mapping_file, "pub const MAPPING: &[Block] = &[")?;

        for block in &blocks {
            // Verify the state ids match the location of each state in the
            // mapping array.
            let state_count = block
                .props
                .iter()
                .map(|p| &p.values)
                .fold(1, |acc, item| acc * item.len());
            assert!(
                block.min_state_id == current_state_id,
                "Incorrect min state for {} in {}, expected {} but found {}",
                block.name,
                version_str,
                block.min_state_id,
                current_state_id
            );
            assert!(
                block.max_state_id == current_state_id + state_count - 1,
                "Incorrect max state for {} in {}, expected {} but found {}",
                block.name,
                version_str,
                block.min_state_id,
                current_state_id + state_count - 1
            );
            current_state_id += state_count;

            write!(mapping_file, "{}", block)?;
        }

        writeln!(mapping_file, "];")?;
    }

    {
        let mut versions_file = File::create(output_dir.join("versions/mod.rs")).unwrap();
        writeln!(
            versions_file,
            "// This file was autogenerated by `generate_blocks`."
        )?;
        writeln!(versions_file, "// Do not modify this file by hand.\n")?;
        writeln!(versions_file, "use crate::Block;")?;
        writeln!(versions_file, "use shared::Version;\n")?;
        writeln!(versions_file, "pub mod legacy;")?;

        for (version, _) in supported_versions {
            writeln!(versions_file, "mod v{};", version.replace('.', "_"))?;
        }

        writeln!(versions_file)?;
        writeln!(
            versions_file,
            "pub fn get_block_mapping(version: Version) -> &'static [Block] {{"
        )?;
        writeln!(versions_file, "    match version {{")?;

        for (version_str, version) in supported_versions {
            let full = version_str.replace('.', "_");
            writeln!(
                versions_file,
                "        Version::{version:?} => v{full}::MAPPING,"
            )?;
        }

        writeln!(versions_file, "        _ => unreachable!(),")?;
        writeln!(versions_file, "    }}")?;
        writeln!(versions_file, "}}")?;
    }

    Ok(())
}
