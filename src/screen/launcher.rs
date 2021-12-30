// Copyright 2016 Matthew Collins
// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs;
use std::sync::Arc;

use crate::auth;
use crate::paths;
use crate::protocol;
use crate::ui;

use crate::render::Renderer;
use crate::screen::{Screen, ScreenSystem, ServerList};
use crate::settings::BACKGROUND_IMAGE;
use crate::ui::Container;
use leafish_protocol::protocol::login::Account;
use parking_lot::Mutex;
use rand::Rng;
use rfd::FileDialog;
use std::fs::File;
use std::io::{Read, Write};
use std::rc::Rc;

pub struct Launcher {
    rendered_accounts: Vec<RenderAccount>,
    options: Option<ui::ButtonRef>,
    disclaimer: Option<ui::TextRef>,
    accounts: Arc<Mutex<Vec<Account>>>,
    add: Option<ui::ButtonRef>,
    background_selection: Option<ui::ButtonRef>,
    screen_sys: Arc<ScreenSystem>,
    active_account: Arc<Mutex<Option<Account>>>,
}

impl Clone for Launcher {
    fn clone(&self) -> Self {
        Launcher::new(
            self.accounts.clone(),
            self.screen_sys.clone(),
            self.active_account.clone(),
        )
    }
}

struct RenderAccount {
    _head_picture: Option<ui::ImageRef>,
    _entry_back: Option<ui::ImageRef>,
    _account_name: Option<ui::TextRef>,
}

impl Launcher {
    pub fn new(
        accounts: Arc<Mutex<Vec<Account>>>,
        screen_sys: Arc<ScreenSystem>,
        active_account: Arc<Mutex<Option<Account>>>,
    ) -> Self {
        Launcher {
            rendered_accounts: vec![],
            options: None,
            disclaimer: None,
            accounts,
            add: None,
            background_selection: None,
            screen_sys,
            active_account,
        }
    }
}

impl super::Screen for Launcher {
    fn on_active(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<Renderer>,
        ui_container: &mut ui::Container,
    ) {
        // Options menu
        let options = ui::ButtonBuilder::new()
            .position(5.0, 25.0)
            .size(40.0, 40.0)
            .draw_index(1)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Right)
            .create(ui_container);
        {
            let mut options = options.borrow_mut();
            ui::ImageBuilder::new()
                .texture("leafish:gui/cog")
                .position(0.0, 0.0)
                .size(40.0, 40.0)
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *options);
            options.add_click_func(|_, game| {
                game.screen_sys
                    .clone()
                    .add_screen(Box::new(super::SettingsMenu::new(game.vars.clone(), false)));
                true
            });
        }
        self.options.replace(options);

        // Disclaimer
        let disclaimer = ui::TextBuilder::new()
            .text("Not affiliated with Mojang/Minecraft")
            .position(5.0, 5.0)
            .colour((255, 200, 200, 255))
            .draw_index(1)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Right)
            .create(ui_container);
        self.disclaimer.replace(disclaimer);

        // Add a new server to the list
        let add = ui::ButtonBuilder::new()
            .position(200.0, -50.0 - 15.0)
            .size(100.0, 30.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .draw_index(2)
            .create(ui_container);
        {
            let mut add = add.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Add")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *add);
            add.add_text(txt);
            let accounts = self.accounts.clone();
            let screen_sys = self.screen_sys.clone();
            add.add_click_func(move |_, game| {
                let accounts = accounts.clone();
                let screen_sys = screen_sys.clone();
                game.screen_sys
                    .clone()
                    .add_screen(Box::new(super::login::Login::new(
                        Arc::new(move |account| {
                            let accounts = accounts.clone();
                            let screen_sys = screen_sys.clone();
                            if let Some(account) = account {
                                accounts.lock().push(account);
                            }
                            screen_sys.pop_screen();
                            save_accounts(&*accounts.lock());
                        }),
                        game.vars.clone(),
                    )));
                true
            })
        }
        self.add.replace(add);
        let background_selection = ui::ButtonBuilder::new()
            .position(10.0, 25.0)
            .size(200.0, 30.0)
            .alignment(ui::VAttach::Bottom, ui::HAttach::Left)
            .draw_index(2)
            .create(ui_container);
        {
            let mut background_selection = background_selection.borrow_mut();
            let txt = ui::TextBuilder::new()
                .text("Select background")
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *background_selection);
            background_selection.add_text(txt);
            background_selection.add_click_func(move |_, game| {
                let files = FileDialog::new()
                    .add_filter("picture", &["png", "jpg", "img"])
                    .set_directory(
                        dirs::picture_dir()
                            .map_or(String::new(), |x| x.as_path().to_str().unwrap().to_string()),
                    )
                    .pick_file();
                if let Some(files) = files {
                    let file_name = files.as_path().to_str().unwrap();
                    game.vars.set(BACKGROUND_IMAGE, file_name.to_string());
                }
                true
            })
        }
        self.background_selection.replace(background_selection);
        let mut offset = 0.0;
        let accounts = self.accounts.clone();
        let accounts = accounts.lock();
        let iter = accounts.iter().cloned();
        for account in iter.enumerate() {
            let idx = account.0;
            let account = account.1;
            let account_name_text = account.name.clone();
            let account_account_name_text = account.verification_tokens.get(0).unwrap().clone();
            let account_password_text = account.verification_tokens.get(1).unwrap().clone();
            let account_type = account.account_type.clone();
            // Everything is attached to this
            let back = ui::ImageBuilder::new()
                .texture("leafish:solid")
                .colour((0, 0, 0, 150))
                .position(0.0, offset * 105.0)
                .size(500.0, 100.0)
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .create(ui_container);
            {
                let account_password_text = account_password_text.clone();
                let account_account_name_text = account_account_name_text.clone();
                let account_type = account_type.clone();
                let accounts = self.accounts.clone();
                let mut back = back.borrow_mut();
                let active_account = self.active_account.clone();
                back.add_click_func(move |_, game| {
                    let accounts = accounts.clone();
                    let account_type = account_type.clone();
                    let idx = idx;
                    let mut client_token = game.vars.get(auth::AUTH_CLIENT_TOKEN).clone();
                    if client_token.is_empty() {
                        client_token = std::iter::repeat(())
                            .map(|()| {
                                rand::thread_rng().sample(&rand::distributions::Alphanumeric)
                                    as char
                            })
                            .take(20)
                            .collect();
                        game.vars.set(auth::AUTH_CLIENT_TOKEN, client_token);
                    }
                    let client_token = game.vars.get(auth::AUTH_CLIENT_TOKEN).clone();
                    let result = protocol::login::ACCOUNT_IMPLS
                        .get(&account.account_type)
                        .unwrap()
                        .value()
                        .refresh(account.clone(), &*client_token);
                    if result.is_ok() {
                        active_account.clone().lock().replace(result.ok().unwrap());
                        game.screen_sys
                            .clone()
                            .add_screen(Box::new(ServerList::new(None)));
                    } else {
                        println!(
                            "An error occoured while attempting to login {}",
                            result.err().unwrap()
                        );
                        game.screen_sys.clone().add_screen(Box::new(
                            super::edit_account::EditAccountEntry::new(
                                Some((
                                    account_account_name_text.clone(),
                                    account_password_text.clone(),
                                )),
                                Rc::new(move |game, name, password| {
                                    let client_token =
                                        game.vars.get(auth::AUTH_CLIENT_TOKEN).clone();
                                    let account = crate::screen::login::try_login(
                                        false,
                                        name,
                                        None,
                                        password,
                                        account_type.clone(),
                                        client_token,
                                    );
                                    let accounts = accounts.clone();
                                    let mut accounts = accounts.lock();
                                    match account {
                                        Ok(account) => {
                                            drop(std::mem::replace(&mut (*accounts)[idx], account));
                                            save_accounts(&*accounts);
                                        }
                                        Err(err) => {
                                            println!(
                                                "An error occoured while modifying the account! {}",
                                                err
                                            );
                                            // TODO: Display this!
                                        }
                                    }
                                }),
                            ),
                        ));
                    }
                    true
                });
                back.add_hover_func(|el, hovered, _| {
                    if hovered {
                        el.colour = (0, 0, 0, 100);
                    } else {
                        el.colour = (0, 0, 0, 150);
                    }
                    true
                });
            }
            let account_name = ui::TextBuilder::new()
                .text(account_name_text.clone())
                .position(0.0, -32.5)
                .colour((200, 200, 200, 255))
                .draw_index(1)
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .attach(&mut *back.borrow_mut());
            let head = ui::ImageBuilder::new()
                .texture("none") // TODO: Load the actual head image!
                .position(-200.0, offset * 105.0)
                .size(85.0, 85.0)
                .colour((0, 0, 0, 255))
                .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                .create(ui_container);
            // Delete entry button
            let delete_entry = ui::ButtonBuilder::new()
                .position(0.0, 0.0)
                .size(25.0, 25.0)
                .alignment(ui::VAttach::Bottom, ui::HAttach::Right)
                .attach(&mut *back.borrow_mut());
            {
                let mut btn = delete_entry.borrow_mut();
                let txt = ui::TextBuilder::new()
                    .text("X")
                    .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                    .attach(&mut *btn);
                btn.add_text(txt);
                let accounts = self.accounts.clone();
                btn.add_click_func(move |_, game| {
                    let accounts = accounts.clone();
                    let idx = idx;
                    game.screen_sys.clone().add_screen(Box::new(
                        super::confirm_box::ConfirmBox::new(
                            String::from("Do you want to delete this account?"),
                            Rc::new(|game| {
                                game.screen_sys.pop_screen();
                            }),
                            Rc::new(move |game| {
                                accounts.clone().lock().remove(idx);
                                save_accounts(&accounts.clone().lock());
                                game.screen_sys.pop_screen();
                            }),
                        ),
                    ));
                    true
                })
            }

            // Edit entry button
            let edit_entry = ui::ButtonBuilder::new()
                .position(25.0, 0.0)
                .size(25.0, 25.0)
                .alignment(ui::VAttach::Bottom, ui::HAttach::Right)
                .attach(&mut *back.borrow_mut());
            {
                let mut btn = edit_entry.borrow_mut();
                let txt = ui::TextBuilder::new()
                    .text("E")
                    .alignment(ui::VAttach::Middle, ui::HAttach::Center)
                    .attach(&mut *btn);
                btn.add_text(txt);
                let aname = account_name_text.clone();
                let apw = account_password_text;
                let account_type = account_type.clone();
                let accounts = self.accounts.clone();
                btn.add_click_func(move |_, game| {
                    let accounts = accounts.clone();
                    let account_type = account_type.clone();
                    let idx = idx;
                    game.screen_sys.clone().add_screen(Box::new(
                        super::edit_account::EditAccountEntry::new(
                            Some((aname.clone(), apw.clone())),
                            Rc::new(move |game, name, password| {
                                let client_token = game.vars.get(auth::AUTH_CLIENT_TOKEN).clone();
                                let account = crate::screen::login::try_login(
                                    false,
                                    name,
                                    None,
                                    password,
                                    account_type.clone(),
                                    client_token,
                                );
                                let accounts = accounts.clone();
                                let mut accounts = accounts.lock();
                                match account {
                                    Ok(account) => {
                                        drop(std::mem::replace(&mut (*accounts)[idx], account));
                                        save_accounts(&*accounts);
                                    }
                                    Err(err) => {
                                        println!(
                                            "An error occoured while modifying the account! {}",
                                            err
                                        );
                                        // TODO: Display this!
                                    }
                                }
                            }),
                        ),
                    ));
                    true
                })
            }
            self.rendered_accounts.push(RenderAccount {
                _head_picture: Some(head),
                _entry_back: Some(back),
                _account_name: Some(account_name),
            });
            offset += 1.0;
        }
    }

    fn on_deactive(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<Renderer>,
        _ui_container: &mut ui::Container,
    ) {
        // Clean up
        self.options.take();
        self.disclaimer.take();
        self.rendered_accounts.clear();
        self.add.take();
        self.background_selection.take();
    }

    fn tick(
        &mut self,
        _screen_sys: &ScreenSystem,
        _renderer: Arc<Renderer>,
        _ui_container: &mut ui::Container,
        _: f64,
    ) {
    }

    fn on_resize(
        &mut self,
        screen_sys: &ScreenSystem,
        renderer: Arc<Renderer>,
        ui_container: &mut Container,
    ) {
        self.on_deactive(screen_sys, renderer.clone(), ui_container);
        self.on_active(screen_sys, renderer, ui_container);
    }

    /*
    fn on_scroll(&mut self, _: f64, y: f64) {
        if self.displayed_accounts.is_empty() {
            return;
        }
        let mut diff = y / 1.0;
        {
            let last = self.displayed_accounts.last().unwrap();
            if last.offset + diff <= 2.0 {
                diff = 2.0 - last.offset;
            }
            let first = self.displayed_accounts.first().unwrap();
            if first.offset + diff >= 0.0 {
                diff = -first.offset;
            }
        }

        for s in &mut self.displayed_accounts {
            s.offset += diff;
            s.update_position();
        }
    }*/

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}

fn save_accounts(accounts: &[Account]) {
    let mut file = File::create(paths::get_config_dir().join("accounts.cfg")).unwrap();
    let json = serde_json::to_string(accounts).unwrap();
    file.write_all(json.as_bytes()).unwrap();
}

pub fn load_accounts() -> Option<Vec<Account>> {
    if let Ok(mut file) = fs::File::open(paths::get_config_dir().join("accounts.cfg")) {
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        let accounts: Option<Vec<Account>> = serde_json::from_str(&*content).ok();
        return accounts;
    }
    None
}
