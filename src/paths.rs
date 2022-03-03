// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs;
use std::path::PathBuf;

fn get_dir(dirtype: Option<PathBuf>) -> PathBuf {
    match dirtype {
        Some(path) => {
            let mut path = path;
            path.push("leafish");
            if !path.exists() {
                fs::create_dir_all(path.clone()).unwrap();
            }
            path
        }
        None => panic!("Unsupported platform"),
    }
}

pub fn get_config_dir() -> PathBuf {
    get_dir(dirs::config_dir())
}

pub fn get_cache_dir() -> PathBuf {
    get_dir(dirs::cache_dir())
}

pub fn get_data_dir() -> PathBuf {
    get_dir(dirs::data_dir())
}
