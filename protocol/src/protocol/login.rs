use crate::protocol::UUID;
use dashmap::DashMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub trait AccountImpl {
    fn login(&self, username: &str, password: &str, token: &str) -> Result<Account, super::Error>;

    fn join_server(
        &self,
        account: &Account,
        server_id: &str,
        shared_key: &[u8],
        public_key: &[u8],
    ) -> Result<(), super::Error>;

    fn refresh(&self, account: Account, token: &str) -> Result<Account, super::Error>;

    fn append_head_img_data(&self, account: &mut Account) -> Result<(), super::Error>;
}

#[derive(Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    pub uuid: Option<String>,
    pub verification_tokens: Vec<String>, // this represents the verification tokens used to verify the account, such as hashed passwords, actual tokens, etc
    pub head_img_data: Option<Vec<u8>>,
    pub account_type: AccountType,
}

impl Account {
    pub fn new(name: String, uuid: Option<String>, account_type: AccountType) -> Self {
        Account {
            name,
            uuid,
            verification_tokens: vec![],
            head_img_data: None,
            account_type,
        }
    }

    /// Whether the profile is complete(not head-wise)
    pub fn is_complete(&self) -> bool {
        !self.name.is_empty() && self.uuid.is_some() && !self.verification_tokens.is_empty()
    }

    pub fn join_server(
        &self,
        server_id: &str,
        shared_key: &[u8],
        public_key: &[u8],
    ) -> Result<(), super::Error> {
        ACCOUNT_IMPLS
            .clone()
            .get(&self.account_type)
            .unwrap()
            .clone()
            .join_server(&self, server_id, shared_key, public_key)
    }

    pub fn refresh(self, token: &str) -> Result<Account, super::Error> {
        ACCOUNT_IMPLS
            .clone()
            .get(&self.account_type)
            .unwrap()
            .clone()
            .refresh(self, token)
    }

    pub fn append_head_img_data(&mut self) -> Result<(), super::Error> {
        ACCOUNT_IMPLS
            .clone()
            .get(&self.account_type)
            .unwrap()
            .clone()
            .append_head_img_data(self)
    }

    pub fn login(
        username: &str,
        password: &str,
        token: &str,
        account_type: AccountType,
    ) -> Result<Account, super::Error> {
        ACCOUNT_IMPLS
            .clone()
            .get(&account_type)
            .unwrap()
            .clone()
            .login(username, password, token)
    }
}

impl Clone for Account {
    fn clone(&self) -> Self {
        Account {
            name: self.name.clone(),
            uuid: self.uuid.clone(),
            verification_tokens: self.verification_tokens.to_vec(),
            head_img_data: self.head_img_data.as_ref().map(|x| x.to_vec()),
            account_type: AccountType::Mojang,
        }
    }
}

lazy_static! {
    pub static ref ACCOUNT_IMPLS: Arc<DashMap<AccountType, Arc<dyn AccountImpl + Send + Sync>>> =
        Arc::new(DashMap::new());
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AccountType {
    Mojang,
    Microsoft,
    Custom(String), // Not implemented yet, this will enable us to support other auth services without implementing every single one specifically
    None,           // aka. unverified or "offline account" (for offline mode servers)
}
