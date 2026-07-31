use std::sync::Mutex;

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::lb::Listen;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS listens (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  listened_at INTEGER NOT NULL,
  artist TEXT NOT NULL,
  title TEXT NOT NULL,
  album TEXT,
  duration INTEGER,
  origin TEXT
);
CREATE TABLE IF NOT EXISTS now_playing (
  origin TEXT PRIMARY KEY,
  artist TEXT NOT NULL,
  title TEXT NOT NULL,
  album TEXT,
  duration INTEGER,
  started_at INTEGER NOT NULL,
  state TEXT NOT NULL,
  position_frozen INTEGER,
  updated_at INTEGER NOT NULL
);
";

const KEEP_LISTENS: u32 = 500;

pub struct Db {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone)]
pub struct NowPlaying {
    pub origin: String,
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration: Option<i64>,
    pub started_at: i64,
    pub state: String,
    pub position_frozen: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct ListenRow {
    pub listened_at: i64,
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration: Option<i64>,
    pub origin: String,
}

impl Db {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn set_now_playing(&self, listen: &Listen, now: i64) {
        let origin = listen.origin();
        let artist = listen.track_metadata.artist_name.as_string();
        let title = listen.track_metadata.track_name.clone();
        let album = listen.track_metadata.release_name.clone();
        let duration = listen.track_metadata.additional_info.duration;
        let playing = listen.is_playing();

        let conn = self.conn.lock().unwrap();
        let existing: Option<(String, i64, Option<i64>)> = conn
            .query_row(
                "SELECT state, started_at, position_frozen
                 FROM now_playing
                 WHERE origin = ?1 AND artist = ?2 AND title = ?3",
                params![origin, artist, title],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        if let Some((state, started_at, position_frozen)) = existing {
            if playing {
                if state == "paused" {
                    let frozen = position_frozen.unwrap_or(0).max(0);
                    conn.execute(
                        "UPDATE now_playing
                         SET started_at = ?1, state = 'playing', position_frozen = NULL, updated_at = ?1
                         WHERE origin = ?2 AND artist = ?3 AND title = ?4",
                        params![now - frozen, origin, artist, title],
                    )
                    .ok();
                } else {
                    conn.execute(
                        "UPDATE now_playing SET updated_at = ?1 WHERE origin = ?2 AND artist = ?3 AND title = ?4",
                        params![now, origin, artist, title],
                    )
                    .ok();
                }
            } else {
                let frozen = (now - started_at).max(0);
                conn.execute(
                    "UPDATE now_playing
                     SET state = 'paused', position_frozen = ?1, updated_at = ?1
                     WHERE origin = ?2 AND artist = ?3 AND title = ?4",
                    params![frozen, origin, artist, title],
                )
                .ok();
            }
        } else if playing {
            conn.execute(
                "INSERT OR REPLACE INTO now_playing
                   (origin, artist, title, album, duration, started_at, state, position_frozen, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'playing', NULL, ?6)",
                params![origin, artist, title, album, duration, now],
            )
            .ok();
        } else {
            conn.execute(
                "INSERT OR REPLACE INTO now_playing
                   (origin, artist, title, album, duration, started_at, state, position_frozen, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'paused', 0, ?6)",
                params![origin, artist, title, album, duration, now],
            )
            .ok();
        }
    }

    pub fn insert_listens(&self, listens: &[Listen], now: i64) {
        let conn = self.conn.lock().unwrap();
        for listen in listens {
            let listened_at = listen.listened_at.unwrap_or(now);
            let origin = listen.origin();
            let artist = listen.track_metadata.artist_name.as_string();
            let title = listen.track_metadata.track_name.clone();
            let album = listen.track_metadata.release_name.clone();
            let duration = listen.track_metadata.additional_info.duration;
            conn.execute(
                "INSERT INTO listens (listened_at, artist, title, album, duration, origin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![listened_at, artist, title, album, duration, origin],
            )
            .ok();
        }
        conn.execute(
            "DELETE FROM listens
             WHERE id NOT IN (SELECT id FROM listens ORDER BY id DESC LIMIT ?1)",
            params![KEEP_LISTENS],
        )
        .ok();
    }

    pub fn latest_now_playing(&self) -> Option<NowPlaying> {
        self.conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT origin, artist, title, album, duration, started_at, state, position_frozen, updated_at
                 FROM now_playing ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| {
                    Ok(NowPlaying {
                        origin: row.get(0)?,
                        artist: row.get(1)?,
                        title: row.get(2)?,
                        album: row.get(3)?,
                        duration: row.get(4)?,
                        started_at: row.get(5)?,
                        state: row.get(6)?,
                        position_frozen: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            )
            .ok()
    }

    pub fn list_listens(&self, limit: u32) -> Vec<ListenRow> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT listened_at, artist, title, album, duration, origin
             FROM listens ORDER BY id DESC LIMIT ?1",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Vec::new(),
        };
        let iter = match stmt.query_map(params![limit], |row| {
            Ok(ListenRow {
                listened_at: row.get(0)?,
                artist: row.get(1)?,
                title: row.get(2)?,
                album: row.get(3)?,
                duration: row.get(4)?,
                origin: row.get(5)?,
            })
        }) {
            Ok(iter) => iter,
            Err(_) => return Vec::new(),
        };
        iter.flatten().collect()
    }
}
