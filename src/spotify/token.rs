use std::{path::PathBuf, time::SystemTime};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: String,
    pub expires: SystemTime,
}

pub trait TokenStorage {
    fn get(&self) -> Option<Token>;
    fn update(&mut self, token: Token);
}

#[derive(Default)]
pub struct InMemoryTokenStorage {
    token: Option<Token>,
}

impl TokenStorage for InMemoryTokenStorage {
    fn get(&self) -> Option<Token> {
        self.token.clone()
    }

    fn update(&mut self, token: Token) {
        self.token = Some(token);
    }
}

pub struct FileTokenStorage {
    path: PathBuf,
    memory: InMemoryTokenStorage,
}

impl FileTokenStorage {
    fn load(path: &PathBuf) -> anyhow::Result<Token> {
        let file = std::fs::read_to_string(path).context("error reading token file")?;
        let token: Token = serde_json::from_str(&file).context("error parsing token")?;
        Ok(token)
    }

    pub fn load_or_create(path: PathBuf) -> anyhow::Result<Self> {
        let memory = match Self::load(&path) {
            Ok(token) => InMemoryTokenStorage { token: Some(token) },
            Err(_) => InMemoryTokenStorage::default(),
        };
        Ok(FileTokenStorage { path, memory })
    }
}

impl TokenStorage for FileTokenStorage {
    fn get(&self) -> Option<Token> {
        self.memory.get()
    }

    fn update(&mut self, token: Token) {
        let json = serde_json::to_string(&token).unwrap();
        std::fs::write(&self.path, json).unwrap();
        self.memory.update(token);
    }
}
