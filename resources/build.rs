// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::env;
use std::fs;
use std::io::BufWriter;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir);

    let base = Path::new("assets");
    let mut out = Vec::new();
    build_map(&mut out, &base);

    let mut file = BufWriter::new(fs::File::create(&dest.join("resources.rs")).unwrap());
    writeln!(
        file,
        "pub fn get_file(name: &str) -> Option<&'static [u8]> {{"
    )
    .unwrap();
    writeln!(file, "    match name {{").unwrap();
    for path in &out {
        let absolute_path = path.canonicalize().unwrap();

        let absolute = absolute_path.to_str().unwrap().replace("\\", "\\\\");
        let relative = path.to_str().unwrap().replace("\\", "/");

        writeln!(
            file,
            "        {:?} => Some(include_bytes!(\"{}\")),",
            relative, absolute
        )
        .unwrap();
    }
    write!(file, "        _ => None\n    }}\n}}\n").unwrap();
}

fn build_map(out: &mut Vec<PathBuf>, path: &Path) {
    let files = fs::read_dir(path).unwrap();
    for entry in files {
        let entry = entry.unwrap();
        if fs::metadata(entry.path()).unwrap().is_dir() {
            build_map(out, &entry.path());
        } else {
            out.push(entry.path());
        }
    }
}
