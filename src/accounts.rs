use std::{fs::{self, File}, io::{Read, Write}};

use leafish_protocol::protocol::login::{Account, AccountType};

use crate::paths;

pub fn save_accounts(accounts: &[Account]) {
    let mut file = File::create(paths::get_config_dir().join("accounts.cfg")).unwrap();
    // filter out microsoft accounts as these will become invalid after ~1 day, so the launcher has to
    // provide us with a fresh token on startup
    let accounts = accounts
        .iter()
        .filter(|account| account.account_type != AccountType::Microsoft)
        .collect::<Vec<_>>();
    let json = serde_json::to_string(&accounts).unwrap();
    file.write_all(json.as_bytes()).unwrap();
}

pub fn load_accounts() -> Option<Vec<Account>> {
    if let Ok(mut file) = fs::File::open(paths::get_config_dir().join("accounts.cfg")) {
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        let accounts: Option<Vec<Account>> = serde_json::from_str(&content).ok();
        return accounts;
    }
    None
}