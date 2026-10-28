// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! LevelDB adapter for the Merkle Patricia Trie using rusty-leveldb (pure Rust).

use eth_trie::DB;
use rusty_leveldb::{Options, DB as LevelDB};
use std::path::Path;
use std::sync::Mutex;

pub struct LevelDbAdapter {
    db: Mutex<LevelDB>,
}

impl LevelDbAdapter {
    pub fn open(path: &Path) -> Result<Self, String> {
        let mut opt = Options::default();
        opt.create_if_missing = true;
        let db = LevelDB::open(path, opt).map_err(|e| format!("{:?}", e))?;
        Ok(LevelDbAdapter { db: Mutex::new(db) })
    }
}

impl DB for LevelDbAdapter {
    type Error = std::io::Error;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        let mut db = self.db.lock().unwrap();
        Ok(db.get(key))
    }

    fn insert(&self, key: &[u8], value: Vec<u8>) -> Result<(), Self::Error> {
        let mut db = self.db.lock().unwrap();
        db.put(key, &value)
            .map_err(|e| std::io::Error::other(format!("{:?}", e)))?;
        db.flush()
            .map_err(|e| std::io::Error::other(format!("{:?}", e)))?;
        Ok(())
    }

    fn remove(&self, key: &[u8]) -> Result<(), Self::Error> {
        let mut db = self.db.lock().unwrap();
        db.delete(key)
            .map_err(|e| std::io::Error::other(format!("{:?}", e)))?;
        db.flush()
            .map_err(|e| std::io::Error::other(format!("{:?}", e)))?;
        Ok(())
    }

    fn flush(&self) -> Result<(), Self::Error> {
        let mut db = self.db.lock().unwrap();
        db.flush()
            .map_err(|e| std::io::Error::other(format!("{:?}", e)))
    }
}
