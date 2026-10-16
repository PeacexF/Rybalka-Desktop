//! Save game in SQLite (`app_data_dir/save.sqlite`).
//!
//! Schema versions are tracked with `PRAGMA user_version`; every entry in
//! [`MIGRATIONS`] upgrades the schema by one version and runs exactly once.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use rybalka_core::profile::Profile;

const MIGRATIONS: &[&str] = &[
    // 1: the profile as one JSON document.
    "CREATE TABLE profile (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        json TEXT NOT NULL,
        saved_at INTEGER NOT NULL
    );",
];

pub struct Store {
    conn: Connection,
}

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("сохранение: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("сохранение повреждено: {0}")]
    Json(#[from] serde_json::Error),
    #[error("сохранение от более новой версии игры (схема {0})")]
    TooNew(i64),
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, SaveError> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::migrate(Connection::open(path)?)
    }

    #[cfg(test)]
    fn in_memory() -> Result<Self, SaveError> {
        Self::migrate(Connection::open_in_memory()?)
    }

    fn migrate(mut conn: Connection) -> Result<Self, SaveError> {
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > MIGRATIONS.len() as i64 {
            return Err(SaveError::TooNew(version));
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i as i64 + 1)?;
            tx.commit()?;
        }
        Ok(Self { conn })
    }

    pub fn has_save(&self) -> Result<bool, SaveError> {
        Ok(self
            .conn
            .query_row("SELECT 1 FROM profile WHERE id = 1", [], |_| Ok(()))
            .optional()?
            .is_some())
    }

    pub fn load(&self) -> Result<Option<Profile>, SaveError> {
        let json: Option<String> = self
            .conn
            .query_row("SELECT json FROM profile WHERE id = 1", [], |r| r.get(0))
            .optional()?;
        Ok(json.map(|j| serde_json::from_str(&j)).transpose()?)
    }

    pub fn save(&self, p: &Profile) -> Result<(), SaveError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        self.conn.execute(
            "INSERT INTO profile (id, json, saved_at) VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET json = excluded.json, saved_at = excluded.saved_at",
            params![serde_json::to_string(p)?, now],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_the_profile() {
        let s = Store::in_memory().unwrap();
        assert!(!s.has_save().unwrap());
        assert_eq!(s.load().unwrap(), None);
        let mut p = Profile {
            money: 123.0,
            ..Profile::default()
        };
        p.items.insert("worm".into(), 7);
        s.save(&p).unwrap();
        p.money = 50.0;
        s.save(&p).unwrap();
        assert!(s.has_save().unwrap());
        assert_eq!(s.load().unwrap(), Some(p));
    }

    #[test]
    fn migrations_run_once_and_newer_saves_are_refused() {
        let dir = std::env::temp_dir().join(format!("rybalka-save-{}", std::process::id()));
        let path = dir.join("save.sqlite");
        let _ = std::fs::remove_file(&path);
        Store::open(&path)
            .unwrap()
            .save(&Profile::default())
            .unwrap();
        // Reopening keeps the data and doesn't re-run migrations.
        assert!(Store::open(&path).unwrap().has_save().unwrap());

        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        drop(conn);
        assert!(matches!(Store::open(&path), Err(SaveError::TooNew(99))));
        let _ = std::fs::remove_dir_all(dir);
    }
}
