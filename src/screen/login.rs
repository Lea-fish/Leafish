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

use std::cell::Cell;
use std::rc::Rc;
use std::sync::{mpsc, Arc};
use std::thread;

use rand::{self, Rng};

use crate::auth;
use crate::console;
use crate::protocol;
use crate::protocol::mojang;
use crate::render;
use crate::screen::Screen;
use crate::settings;
use crate::ui;
use leafish_protocol::protocol::{UUID, Error};
use std::str::FromStr;
use std::ops::Deref;
use leafish_protocol::protocol::login::{Account, AccountType};

pub struct Login {
    elements: Option<UIElements>,
    callback: Arc<dyn Fn(Option<Account>)>,
}

impl Clone for Login {
    fn clone(&self) -> Self {
        Login {
            elements: None,
            callback: self.callback.clone(),
        }
    }
}

struct UIElements {
    logo: ui::logo::Logo,

    login_btn: ui::ButtonRef,
    login_btn_text: ui::TextRef,
    login_error: ui::TextRef,
    username_txt: ui::TextBoxRef,
    password_txt: ui::TextBoxRef,
    _disclaimer: ui::TextRef,
    try_login: Rc<Cell<bool>>,
    refresh: bool,
    login_res: Option<mpsc::Receiver<Result<Account, protocol::Error>>>,
}

impl Login {
    pub fn new(callback: Arc<dyn Fn(Option<Account>)>) -> Self {
        Login {
            elements: None,
            callback,
        }
    }
}

impl super::Screen for Login {
    fn on_active(&mut self, renderer: &mut render::Renderer, ui_container: &mut ui::Container) {
        let logo = ui::logo::Logo::new(renderer.resources.clone(), ui_container);

        let try_login = Rc::new(Cell::new(false));

        // Login
        let login_btn = ui::ButtonBuilder::new()
            .position(0.0, 100.0)
            .size(400.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        let login_btn_text = ui::TextBuilder::new()
            .text("Login")
            .position(0.0, 0.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .attach(&mut *login_btn.borrow_mut());
        {
            let mut btn = login_btn.borrow_mut();
            btn.add_text(login_btn_text.clone());
            let tl = try_login.clone();
            btn.add_click_func(move |_, _| {
                tl.set(true);
                true
            });
        }

        // Login Error
        let login_error = ui::TextBuilder::new()
            .text("")
            .position(0.0, 150.0)
            .colour((255, 50, 50, 255))
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);

        // Username
        let username_txt = ui::TextBoxBuilder::new()
            .position(0.0, -20.0)
            .size(400.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .create(ui_container);
        ui::TextBox::make_focusable(&username_txt, ui_container);
        ui::TextBuilder::new()
            .text("Username/Email:")
            .position(0.0, -18.0)
            .attach(&mut *username_txt.borrow_mut());

        // Password
        let password_txt = ui::TextBoxBuilder::new()
            .position(0.0, 40.0)
            .size(400.0, 40.0)
            .alignment(ui::VAttach::Middle, ui::HAttach::Center)
            .password(true)
            .create(ui_container);
        ui::TextBox::make_focusable(&password_txt, ui_container);
        ui::TextBuilder::new()
            .text("Password:")
            .position(0.0, -18.0)
            .attach(&mut *password_txt.borrow_mut());
        let tl = try_login.clone();
        password_txt.borrow_mut().add_submit_func(move |_, _| {
            tl.set(true);
        });

        // Disclaimer
        let disclaimer = ui::TextBuilder::new()
            .text("Not affiliated with Mojang/Minecraft")
            .position(5.0, 5.0)
            .colour((255, 200, 200, 255))
            .alignment(ui::VAttach::Bottom, ui::HAttach::Right)
            .create(ui_container);

        let refresh = false; // TODO: Detect this!
        try_login.set(refresh);

        self.elements = Some(UIElements {
            logo,
            login_btn,
            login_btn_text,
            login_error,
            try_login,
            refresh,
            login_res: None,

            _disclaimer: disclaimer,

            username_txt,
            password_txt,
        });
    }
    fn on_deactive(&mut self, _renderer: &mut render::Renderer, _ui_container: &mut ui::Container) {
        // Clean up
        self.elements = None
    }

    fn tick(
        &mut self,
        _delta: f64,
        renderer: &mut render::Renderer,
        _ui_container: &mut ui::Container,
    ) -> Option<Box<dyn super::Screen>> {
        let elements = self.elements.as_mut().unwrap();

        if elements.try_login.get() && elements.login_res.is_none() {
            elements.try_login.set(false);
            let (tx, rx) = mpsc::channel();
            elements.login_res = Some(rx);
            elements.login_btn.borrow_mut().disabled = true;
            elements.login_btn_text.borrow_mut().text = "Logging in...".into();
            let client_token = String::new();
            let username = elements.username_txt.borrow().input.clone();
            let password = elements.password_txt.borrow().input.clone();
            let refresh = elements.refresh;
            thread::spawn(move || {
                tx.send(try_login(refresh, username.clone(), client_token, password, AccountType::Mojang));
            });
        }
        let mut done = false;
        if let Some(rx) = elements.login_res.as_ref() {
            if let Ok(res) = rx.try_recv() {
                done = true;
                elements.login_btn.borrow_mut().disabled = false;
                elements.login_btn_text.borrow_mut().text = "Login".into();
                match res {
                    Ok(account) => {
                        self.callback.clone().deref()(Some(account));
                        return None;
                    }
                    Err(err) => {
                        elements.login_error.borrow_mut().text = format!("{}", err);
                    }
                }
            }
        }
        if done {
            elements.login_res = None;
        }

        elements.logo.tick(renderer);
        None
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }
}

fn try_login(refresh: bool, name: String, token: String, password: String, account_type: AccountType) -> Result<Account, Error> {
    try_login_account(refresh, Account {
        name,
        uuid: None,
        verification_tokens: vec![password, token],
        head_img_data: None,
        account_type,
    })
}

static DEFAULT_PW: String = String::new();

fn try_login_account(refresh: bool, account: Account) -> Result<Account, Error> {
    let password = if !account.verification_tokens.is_empty() {
        account.verification_tokens.get(0).unwrap()
    } else {
        &DEFAULT_PW
    };
    if refresh && (account.name.is_empty() || password.is_empty()) { // password is at idx 0 in the verification tokens
        let client_token = account.verification_tokens.get(1).unwrap().clone();
        account.refresh(&*client_token)
    } else {
        let client_token = account.verification_tokens.get(1).unwrap();
        Account::login(&account.name, password, client_token, AccountType::Mojang)
    }
}
