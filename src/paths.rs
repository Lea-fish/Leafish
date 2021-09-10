// Copyright 2021 Bart Ribbers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
