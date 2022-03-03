// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use leafish_blocks::VanillaIDMap;
use std::collections::HashMap;
use std::env;

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
    let block = id_map.by_vanilla_id(id, &HashMap::new());

    println!("{:?}", block);
}
