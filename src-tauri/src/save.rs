//! Save game in SQLite (`app_data_dir/save.sqlite`).
//!
//! Schema versions are tracked with `PRAGMA user_version`; every entry in
//! [`MIGRATIONS`] upgrades the schema by one version and runs exactly once.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use rybalka_core::journal::{CatchLog, JournalEntry, TripLog};
use rybalka_core::profile::Profile;

const MIGRATIONS: &[&str] = &[
    // 1: the profile as one JSON document.
    "CREATE TABLE profile (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        json TEXT NOT NULL,
        saved_at INTEGER NOT NULL
    );",
    // 2: the diary — every landed fish and every finished trip.
    "CREATE TABLE catches (
        id INTEGER PRIMARY KEY,
        at_min REAL NOT NULL,
        fish_id TEXT NOT NULL,
        weight_kg REAL NOT NULL,
        json TEXT NOT NULL
    );
    CREATE INDEX catches_fish ON catches (fish_id, weight_kg);
    CREATE TABLE trips (
        id INTEGER PRIMARY KEY,
        ended_min REAL NOT NULL,
        json TEXT NOT NULL
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

    #[cfg(test)]
    fn save(&mut self, p: &Profile) -> Result<(), SaveError> {
        self.write(Some(p), &[])
    }

    /// Save the profile and append diary entries in one transaction, so the
    /// records in the profile and the catch log never disagree.
    pub fn write(
        &mut self,
        profile: Option<&Profile>,
        journal: &[JournalEntry],
    ) -> Result<(), SaveError> {
        let tx = self.conn.transaction()?;
        if let Some(p) = profile {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64);
            tx.execute(
                "INSERT INTO profile (id, json, saved_at) VALUES (1, ?1, ?2)
                 ON CONFLICT(id) DO UPDATE SET json = excluded.json, saved_at = excluded.saved_at",
                params![serde_json::to_string(p)?, now],
            )?;
        }
        for e in journal {
            match e {
                JournalEntry::Catch(c) => tx.execute(
                    "INSERT INTO catches (at_min, fish_id, weight_kg, json) VALUES (?1, ?2, ?3, ?4)",
                    params![c.at_min, c.fish_id, c.weight_kg, serde_json::to_string(c)?],
                )?,
                JournalEntry::Trip(t) => tx.execute(
                    "INSERT INTO trips (ended_min, json) VALUES (?1, ?2)",
                    params![t.ended_min, serde_json::to_string(t)?],
                )?,
            };
        }
        tx.commit()?;
        Ok(())
    }

    /// Start over: forget the diary (the profile is overwritten separately).
    pub fn clear_journal(&self) -> Result<(), SaveError> {
        self.conn
            .execute_batch("DELETE FROM catches; DELETE FROM trips;")?;
        Ok(())
    }

    /// The newest trips and catches, newest first.
    pub fn diary(&self, limit: u32) -> Result<(Vec<TripLog>, Vec<CatchLog>), SaveError> {
        let trips = self.rows("SELECT json FROM trips ORDER BY id DESC LIMIT ?1", limit)?;
        let catches = self.rows("SELECT json FROM catches ORDER BY id DESC LIMIT ?1", limit)?;
        Ok((trips, catches))
    }

    fn rows<T: serde::de::DeserializeOwned>(
        &self,
        sql: &str,
        limit: u32,
    ) -> Result<Vec<T>, SaveError> {
        let mut stmt = self.conn.prepare(sql)?;
        let json = stmt
            .query_map([limit], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        // A row that no longer parses (older format) is skipped, not fatal.
        Ok(json
            .iter()
            .filter_map(|j| serde_json::from_str(j).ok())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_the_profile() {
        let mut s = Store::in_memory().unwrap();
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
    fn diary_entries_are_appended_and_listed_newest_first() {
        use rybalka_core::sim::rod::RodKind;
        use rybalka_core::weather::Weather;
        let mut s = Store::in_memory().unwrap();
        let fish = |kg: f64| {
            JournalEntry::Catch(CatchLog {
                fish_id: "roach".into(),
                fish_name: "Плотва".into(),
                weight_kg: kg,
                trophy: false,
                record: false,
                bait: "Червь".into(),
                rod: RodKind::Float,
                location: "Пруд".into(),
                spot: "Под берёзой".into(),
                at_min: kg * 100.0,
                weather: Weather::Clear,
            })
        };
        let trip = JournalEntry::Trip(TripLog {
            location: "Пруд".into(),
            started_min: 0.0,
            ended_min: 100.0,
            fish: 2,
            kg: 0.5,
            earned: 30.0,
            best_name: Some("Плотва".into()),
            best_kg: 0.3,
            weathers: vec![Weather::Clear],
        });
        s.write(Some(&Profile::default()), &[fish(0.2), fish(0.3), trip])
            .unwrap();
        let (trips, catches) = s.diary(10).unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(catches.len(), 2);
        assert!((catches[0].weight_kg - 0.3).abs() < 1e-9, "newest first");
        s.clear_journal().unwrap();
        assert!(s.diary(10).unwrap().1.is_empty());
        assert!(
            s.has_save().unwrap(),
            "clearing the diary keeps the profile"
        );
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
        let version: i64 = Connection::open(&path)
            .unwrap()
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
        // Reopening keeps the data and doesn't re-run migrations.
        assert!(Store::open(&path).unwrap().has_save().unwrap());

        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        drop(conn);
        assert!(matches!(Store::open(&path), Err(SaveError::TooNew(99))));
        let _ = std::fs::remove_dir_all(dir);
    }
}
