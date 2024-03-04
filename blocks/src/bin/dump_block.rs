use leafish_blocks::VanillaIDMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!(
            "usage: {} protocol_version id\nrun with DEBUG_BLOCKS=1 to dump all",
            args[0]
        );
        return;
    }
    let protocol_version = str::parse::<i32>(&args[1]).unwrap();
    let id = str::parse::<usize>(&args[2]).unwrap();

    let id_map = VanillaIDMap::new(protocol_version);
    let block = id_map.by_vanilla_id(id, &Arc::new(HashMap::new()));

    println!("{:?}", block);
}
