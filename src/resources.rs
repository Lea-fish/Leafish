// Copyright 2016 Matthew Collins
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

extern crate leafish_resources as internal;

use log::warn;

use crate::paths;

use std::collections::HashMap;
use std::fs;
use std::hash::BuildHasherDefault;
use std::io;
use std::path;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::types::hash::FNVHash;
use crate::ui;
use std::fs::File;

// for latest asset indices and a list of them in general https://piston-meta.mojang.com/mc/game/version_manifest_v2.json

/*
const RESOURCES_VERSION: &str = "1.20.4";
const VANILLA_CLIENT_URL: &str =
    "https://piston-data.mojang.com/v1/objects/fd19469fed4a4b4c15b2d5133985f0e3e7816a8a/client.jar";
const ASSET_VERSION: &str = "1.20";
const ASSET_INDEX_URL: &str =
    "https://piston-meta.mojang.com/v1/packages/54c04f81364f5fcb91da8b95ecf146cc396a4afc/1.20.4.json";
*/
const RESOURCES_VERSION: &str = "1.19.2";
const VANILLA_CLIENT_URL: &str =
    "https://piston-data.mojang.com/v1/objects/055b30d860ead928cba3849ba920c88b6950b654/client.jar";
const ASSET_VERSION: &str = "1.19";
const ASSET_INDEX_URL: &str =
    "https://piston-meta.mojang.com/v1/packages/b5c7548ddb9e584e84a5f762da5b78211c715a63/1.19.json";

pub trait Pack: Sync + Send {
    fn open(&self, name: &str) -> Option<Box<dyn io::Read>>;
}

pub struct Manager {
    packs: Vec<Box<dyn Pack>>,
    version: usize,

    vanilla_progress: Arc<Mutex<Progress>>,
    pending_downloads: Arc<AtomicUsize>,
}

pub struct ManagerUI {
    progress_ui: Vec<ProgressUI>,
    num_tasks: isize,
}

struct ProgressUI {
    task_name: String,
    task_file: String,
    position: f64,
    closing: bool,
    progress: f64,

    background: ui::ImageRef,
    progress_bar: ui::ImageRef,
}

struct Progress {
    tasks: Vec<Task>,
}

struct Task {
    task_name: String,
    task_file: String,
    total: u64,
    progress: u64,
}

unsafe impl Sync for Manager {}

impl Manager {
    const LOAD_VANILLA_FLAG: usize = 1 << (usize::BITS as usize - 1);
    const LOAD_ASSETS_FLAG: usize = 1 << (usize::BITS as usize - 2);
    const META_MASK: usize = Self::LOAD_ASSETS_FLAG | Self::LOAD_VANILLA_FLAG;

    pub fn new(
        provided_assets: Option<String>,
        provided_client: Option<String>,
    ) -> (Manager, ManagerUI) {
        let mut m = Manager {
            packs: Vec::new(),
            version: 0,
            vanilla_progress: Arc::new(Mutex::new(Progress { tasks: vec![] })),
            pending_downloads: Arc::new(AtomicUsize::new(1)),
        };
        m.add_pack(Box::new(InternalPack));
        m.download_vanilla(provided_client);
        if let Some(assets) = provided_assets {
            m.preload_assets(assets);
        } else {
            m.download_assets();
        }
        (
            m,
            ManagerUI {
                progress_ui: vec![],
                num_tasks: 0,
            },
        )
    }

    /// Returns the 'version' of the manager. The version is
    /// increase everytime a pack is added or removed.
    pub fn version(&self) -> usize {
        self.version
    }

    pub fn open(&self, plugin: &str, name: &str) -> Option<Box<dyn io::Read>> {
        if plugin == "global" {
            let file = File::open(paths::get_data_dir().join(name));
            if let Ok(file) = file {
                Some(Box::new(file))
            } else {
                None
            }
        } else {
            let path = format!("assets/{}/{}", plugin, name);
            for pack in self.packs.iter().rev() {
                if let Some(val) = pack.open(&path) {
                    return Some(val);
                }
            }

            None
        }
    }

    pub fn open_all(&self, plugin: &str, name: &str) -> Vec<Box<dyn io::Read>> {
        let mut ret = Vec::new();
        let path = format!("assets/{}/{}", plugin, name);
        for pack in self.packs.iter().rev() {
            if let Some(val) = pack.open(&path) {
                ret.push(val);
            }
        }
        ret
    }

    pub fn tick(&mut self, mui: &mut ManagerUI, ui_container: &mut ui::Container, delta: f64) {
        let delta = delta.min(5.0);

        const UI_HEIGHT: f64 = 32.0;

        let pending = self.pending_downloads.load(Ordering::Acquire);
        if pending == 0 {
            // the asset manager has nothing to do!
            return;
        }

        if pending & Self::META_MASK != 0 {
            if pending & Self::LOAD_ASSETS_FLAG != 0 {
                self.pending_downloads
                    .fetch_sub(Self::LOAD_ASSETS_FLAG, Ordering::AcqRel);
                self.load_assets();
            }
            if pending & Self::LOAD_VANILLA_FLAG != 0 {
                self.pending_downloads
                    .fetch_sub(Self::LOAD_VANILLA_FLAG, Ordering::AcqRel);
                self.load_vanilla();
            }
        }

        // Check to see if all downloads have completed
        if pending == 1 {
            self.vanilla_progress.lock().unwrap().tasks.clear();
            mui.num_tasks = 0;
            mui.progress_ui.clear();
            self.pending_downloads.store(0, Ordering::Release);
            return;
        }

        let mut progress = self.vanilla_progress.lock().unwrap();
        progress.tasks.retain(|v| v.progress < v.total);
        // Find out what we have to work with
        for task in &progress.tasks {
            if !mui
                .progress_ui
                .iter()
                .filter(|v| v.task_file == task.task_file)
                .any(|v| v.task_name == task.task_name)
            {
                mui.num_tasks += 1;
                // Add a ui element for it
                let background = ui::ImageBuilder::new()
                    .texture("leafish:solid")
                    .position(0.0, -UI_HEIGHT)
                    .size(350.0, UI_HEIGHT)
                    .colour((0, 0, 0, 100))
                    .draw_index(0xFFFFFF - mui.num_tasks)
                    .alignment(ui::VAttach::Bottom, ui::HAttach::Left)
                    .create(ui_container);

                ui::ImageBuilder::new()
                    .texture("leafish:solid")
                    .position(0.0, 0.0)
                    .size(350.0, 10.0)
                    .colour((0, 0, 0, 200))
                    .attach(&mut *background.borrow_mut());
                ui::TextBuilder::new()
                    .text(&*task.task_name)
                    .position(3.0, 0.0)
                    .scale_x(0.5)
                    .scale_y(0.5)
                    .draw_index(1)
                    .attach(&mut *background.borrow_mut());
                ui::TextBuilder::new()
                    .text(&*task.task_file)
                    .position(3.0, 12.0)
                    .scale_x(0.5)
                    .scale_y(0.5)
                    .draw_index(1)
                    .attach(&mut *background.borrow_mut());

                let progress_bar = ui::ImageBuilder::new()
                    .texture("leafish:solid")
                    .position(0.0, 0.0)
                    .size(0.0, 10.0)
                    .colour((0, 255, 0, 255))
                    .draw_index(2)
                    .alignment(ui::VAttach::Bottom, ui::HAttach::Left)
                    .attach(&mut *background.borrow_mut());

                mui.progress_ui.push(ProgressUI {
                    task_name: task.task_name.clone(),
                    task_file: task.task_file.clone(),
                    position: -UI_HEIGHT,
                    closing: false,
                    progress: 0.0,
                    background,
                    progress_bar,
                });
            }
        }
        for ui in &mut mui.progress_ui {
            if ui.closing {
                continue;
            }
            let mut found = false;
            let mut prog = 1.0;
            for task in progress
                .tasks
                .iter()
                .filter(|v| v.task_file == ui.task_file)
                .filter(|v| v.task_name == ui.task_name)
            {
                found = true;
                prog = task.progress as f64 / task.total as f64;
            }
            let background = ui.background.borrow();
            let progress_bar = ui.progress_bar.borrow();
            // Let the progress bar finish
            if !found
                && (background.y - ui.position).abs() < 0.7 * delta
                && (progress_bar.width - 350.0).abs() < 1.0 * delta
            {
                ui.closing = true;
                ui.position = -UI_HEIGHT;
            }
            ui.progress = prog;
        }
        let mut offset = 0.0;
        for ui in &mut mui.progress_ui {
            if ui.closing {
                continue;
            }
            ui.position = offset;
            offset += UI_HEIGHT;
        }
        // Move elements
        for ui in &mut mui.progress_ui {
            let mut background = ui.background.borrow_mut();
            if (background.y - ui.position).abs() < 0.7 * delta {
                background.y = ui.position;
            } else {
                background.y += (ui.position - background.y).signum() * 0.7 * delta;
            }
            let mut progress_bar = ui.progress_bar.borrow_mut();
            let target_size = (350.0 * ui.progress).min(350.0);
            if (progress_bar.width - target_size).abs() < 1.0 * delta {
                progress_bar.width = target_size;
            } else {
                progress_bar.width +=
                    ((target_size - progress_bar.width).signum() * delta).max(0.0);
            }
        }

        // Clean up dead elements
        mui.progress_ui
            .retain(|v| v.position >= -UI_HEIGHT || !v.closing);
    }

    fn add_pack(&mut self, pck: Box<dyn Pack>) {
        self.packs.push(pck);
        self.version += 1;
    }

    fn load_vanilla(&mut self) {
        let loc = format!("resources-{}", RESOURCES_VERSION);
        let location = paths::get_data_dir().join(loc);
        self.packs.insert(1, Box::new(DirPack { root: location }));
        self.version += 1;
    }

    fn preload_assets(&mut self, path: String) {
        self.packs.insert(1, Box::new(ObjectPack::new(path)));
        self.version += 1;
    }

    fn load_assets(&mut self) {
        self.packs.insert(
            1,
            Box::new(ObjectPack::new(
                paths::get_data_dir()
                    .join(format!("index/{}.json", ASSET_VERSION))
                    .to_str()
                    .unwrap()
                    .to_string(),
            )),
        );
        self.version += 1;
    }

    fn download_assets(&mut self) {
        let loc = paths::get_data_dir().join(format!("index/{}.json", ASSET_VERSION));
        let location = path::Path::new(&loc).to_owned();
        let progress_info = self.vanilla_progress.clone();
        if fs::metadata(&location).is_ok() {
            self.load_assets();
        }
        self.pending_downloads
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let pending_downloads = self.pending_downloads.clone();
        thread::spawn(move || {
            let client = reqwest::blocking::Client::new();
            if fs::metadata(&location).is_err() {
                fs::create_dir_all(location.parent().unwrap()).unwrap();
                let res = client.get(ASSET_INDEX_URL).send().unwrap();

                let length = res
                    .headers()
                    .get(reqwest::header::CONTENT_LENGTH)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();
                Self::add_task(
                    &progress_info,
                    "Downloading Asset Index",
                    &location.to_string_lossy(),
                    length,
                );
                let tmp_file = paths::get_cache_dir().join(format!("index-{}.tmp", ASSET_VERSION));
                {
                    let mut file = fs::File::create(tmp_file.clone()).unwrap();
                    let mut progress = ProgressRead {
                        read: res,
                        progress: &progress_info,
                        task_name: "Downloading Asset Index".into(),
                        task_file: location.to_string_lossy().into_owned(),
                    };
                    io::copy(&mut progress, &mut file).unwrap();
                }
                fs::rename(tmp_file, &location).unwrap();
                // this operation is a combination of `- 1` and `+ LOAD_ASSETS_FLAG`
                #[allow(arithmetic_overflow)]
                pending_downloads.fetch_add(
                    (-1_isize as usize) + Self::LOAD_ASSETS_FLAG,
                    Ordering::AcqRel,
                );
            }

            let file = fs::File::open(&location).unwrap();
            let index: serde_json::Value = serde_json::from_reader(&file).unwrap();
            let root_location = paths::get_data_dir().join("objects");
            let objects = index.get("objects").and_then(|v| v.as_object()).unwrap();
            Self::add_task(
                &progress_info,
                "Downloading Assets",
                &root_location.to_string_lossy(),
                objects.len() as u64,
            );
            for (k, v) in objects {
                let hash = v.get("hash").and_then(|v| v.as_str()).unwrap();
                let hash_path = format!("{}/{}", &hash[..2], hash);
                let location = root_location.join(&hash_path);
                if fs::metadata(&location).is_err() {
                    // ignore errors, as either the asset in question may not be available or
                    // an error might have occoured connecting to the asset server
                    if let Ok(res) = client
                        .get(&format!(
                            "http://resources.download.minecraft.net/{}",
                            hash_path
                        ))
                        .send()
                    {
                        fs::create_dir_all(location.parent().unwrap()).unwrap();
                        let length = v.get("size").and_then(|v| v.as_u64()).unwrap();
                        Self::add_task(&progress_info, "Downloading Asset", k, length);
                        let mut tmp_file = location.to_owned();
                        tmp_file.set_file_name(format!("{}.tmp", hash));
                        {
                            let mut file = fs::File::create(&tmp_file).unwrap();
                            let mut progress = ProgressRead {
                                read: res,
                                progress: &progress_info,
                                task_name: "Downloading Asset".into(),
                                task_file: k.to_owned(),
                            };
                            io::copy(&mut progress, &mut file).unwrap();
                        }
                        fs::rename(&tmp_file, &location).unwrap();
                    } else {
                        warn!("The asset \"{k}\" (hash: {hash}) could not be downloaded");
                    }
                    Self::add_task_progress(
                        &progress_info,
                        "Downloading Assets",
                        &root_location.to_string_lossy(),
                        1,
                    );
                }
            }
        });
    }

    fn download_vanilla(&mut self, provided_client: Option<String>) {
        let loc = paths::get_data_dir().join(format!("resources-{}", RESOURCES_VERSION));
        let location = path::Path::new(&loc);
        // check if there are already preexisting assets we can use
        if fs::metadata(location.join("leafish.assets")).is_ok() {
            self.load_vanilla();
            return;
        }

        // check if there are no preexisting assets, but the bootstrap provided us with a file
        // we can extract the assets from
        if let Some(provided_client) = provided_client.as_ref() {
            // FIXME: this is only temporary, don't block main thread in the future and give some feedback to the user instead!
            Self::unpack_assets(&self.vanilla_progress, &provided_client);
            self.load_vanilla();
            return;
        }

        self.pending_downloads.fetch_add(1, Ordering::AcqRel);

        let progress_info = self.vanilla_progress.clone();
        let pending_downloads = self.pending_downloads.clone();
        thread::spawn(move || {
            let client = reqwest::blocking::Client::new();
            let res = client.get(VANILLA_CLIENT_URL).send().unwrap();
            let tmp_file_path = paths::get_cache_dir().join(format!("{}.tmp", RESOURCES_VERSION));
            let mut file = fs::File::create(tmp_file_path.clone()).unwrap();

            let length = res
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .unwrap()
                .to_str()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            let task_file_path =
                paths::get_data_dir().join(format!("resources-{}", RESOURCES_VERSION));
            let task_file = task_file_path.into_os_string().into_string().unwrap();
            Self::add_task(
                &progress_info,
                "Downloading Core Assets",
                &task_file,
                length,
            );
            {
                let mut progress = ProgressRead {
                    read: res,
                    progress: &progress_info,
                    task_name: "Downloading Core Assets".into(),
                    task_file,
                };
                io::copy(&mut progress, &mut file).unwrap();
            }

            Self::unpack_assets(&progress_info, &tmp_file_path.to_str().unwrap().to_string());

            // this operation is a combination of `- 1` and `+ LOAD_VANILLA_FLAG`
            #[allow(arithmetic_overflow)]
            pending_downloads.fetch_add(
                (-1_isize as usize) + Self::LOAD_VANILLA_FLAG,
                Ordering::AcqRel,
            );

            fs::remove_file(paths::get_cache_dir().join(format!("{}.tmp", RESOURCES_VERSION)))
                .unwrap();
        });
    }

    fn unpack_assets(progress_info: &Arc<Mutex<Progress>>, path: &str) {
        // Copy the resources from the zip
        let file = fs::File::open(path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();

        let task_file_path = paths::get_data_dir().join(format!("resources-{}", RESOURCES_VERSION));
        let task_file = task_file_path.into_os_string().into_string().unwrap();
        Self::add_task(
            progress_info,
            "Unpacking Core Assets",
            &task_file,
            zip.len() as u64,
        );

        let loc = paths::get_data_dir().join(format!("resources-{}", RESOURCES_VERSION));
        let location = path::Path::new(&loc);
        let count = zip.len();
        for i in 0..count {
            Self::add_task_progress(progress_info, "Unpacking Core Assets", &task_file, 1);
            let mut file = zip.by_index(i).unwrap();
            if !file.name().starts_with("assets/") {
                continue;
            }
            let path = location.join(file.name());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut out = fs::File::create(path).unwrap();
            io::copy(&mut file, &mut out).unwrap();
        }

        fs::File::create(location.join("leafish.assets")).unwrap(); // Marker file
    }

    fn add_task(progress: &Arc<Mutex<Progress>>, name: &str, file: &str, length: u64) {
        let mut info = progress.lock().unwrap();
        info.tasks.push(Task {
            task_name: name.into(),
            task_file: file.into(),
            total: length,
            progress: 0,
        });
    }

    fn add_task_progress(progress: &Arc<Mutex<Progress>>, name: &str, file: &str, prog: u64) {
        let mut progress = progress.lock().unwrap();
        for task in progress
            .tasks
            .iter_mut()
            .filter(|v| v.task_file == file)
            .filter(|v| v.task_name == name)
        {
            task.progress += prog;
        }
    }
}

struct DirPack {
    root: path::PathBuf,
}

impl Pack for DirPack {
    fn open(&self, name: &str) -> Option<Box<dyn io::Read>> {
        match fs::File::open(self.root.join(name)) {
            Ok(val) => Some(Box::new(val)),
            Err(_) => None,
        }
    }
}

struct InternalPack;

impl Pack for InternalPack {
    fn open(&self, name: &str) -> Option<Box<dyn io::Read>> {
        match internal::get_file(name) {
            Some(val) => Some(Box::new(io::Cursor::new(val))),
            None => None,
        }
    }
}

struct ObjectPack {
    objects: HashMap<String, String, BuildHasherDefault<FNVHash>>,
}

impl ObjectPack {
    fn new(loc: String) -> ObjectPack {
        let location = path::Path::new(&loc);
        let file = fs::File::open(location).unwrap();
        let index: serde_json::Value = serde_json::from_reader(&file).unwrap();
        let objects = index.get("objects").and_then(|v| v.as_object()).unwrap();
        let mut hash_objs = HashMap::with_hasher(BuildHasherDefault::default());
        for (k, v) in objects {
            hash_objs.insert(
                k.clone(),
                v.get("hash").and_then(|v| v.as_str()).unwrap().to_owned(),
            );
        }
        ObjectPack { objects: hash_objs }
    }
}

impl Pack for ObjectPack {
    fn open(&self, name: &str) -> Option<Box<dyn io::Read>> {
        if !name.starts_with("assets/") {
            return None;
        }
        let name = &name["assets/".len()..];
        if let Some(hash) = self.objects.get(name) {
            let root_location = path::Path::new("./objects/");
            let hash_path = format!("{}/{}", &hash[..2], hash);
            let location = root_location.join(hash_path);
            match fs::File::open(location) {
                Ok(val) => Some(Box::new(val)),
                Err(_) => None,
            }
        } else {
            None
        }
    }
}

struct ProgressRead<'a, T> {
    read: T,
    progress: &'a Arc<Mutex<Progress>>,
    task_name: String,
    task_file: String,
}

impl<'a, T: io::Read> io::Read for ProgressRead<'a, T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let size = self.read.read(buf)?;
        Manager::add_task_progress(self.progress, &self.task_name, &self.task_file, size as u64);
        Ok(size)
    }
}
