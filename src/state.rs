use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::lb::Listen;

const KEEP_LISTENS: usize = 30;

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
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

pub struct State {
    listens: VecDeque<ListenRow>,
    now_playing: Option<NowPlaying>,
}

impl State {
    pub fn new() -> Self {
        Self {
            listens: VecDeque::new(),
            now_playing: None,
        }
    }

    pub fn set_now_playing(&mut self, listen: &Listen, now: i64) {
        let origin = listen.origin();
        let artist = listen.track_metadata.artist_name.as_string();
        let title = listen.track_metadata.track_name.clone();
        let album = listen.track_metadata.release_name.clone();
        let duration = listen.track_metadata.additional_info.duration;
        let playing = listen.is_playing();

        let same_track = self.now_playing.as_ref().is_some_and(|np| {
            np.origin == origin && np.artist == artist && np.title == title
        });

        if same_track {
            if let Some(np) = self.now_playing.as_mut() {
                if playing {
                    if np.state == "paused" {
                        let frozen = np.position_frozen.unwrap_or(0).max(0);
                        np.started_at = now - frozen;
                        np.position_frozen = None;
                        np.state = "playing".to_string();
                    }
                    np.updated_at = now;
                } else if np.state == "playing" {
                    np.position_frozen = Some((now - np.started_at).max(0));
                    np.state = "paused".to_string();
                    np.updated_at = now;
                }
            }
        } else {
            self.now_playing = Some(NowPlaying {
                origin,
                artist,
                title,
                album,
                duration,
                started_at: now,
                state: if playing { "playing" } else { "paused" }.to_string(),
                position_frozen: if playing { None } else { Some(0) },
                updated_at: now,
            });
        }
    }

    pub fn insert_listen(&mut self, listen: &Listen, now: i64) {
        let row = ListenRow {
            listened_at: listen.listened_at.unwrap_or(now),
            artist: listen.track_metadata.artist_name.as_string(),
            title: listen.track_metadata.track_name.clone(),
            album: listen.track_metadata.release_name.clone(),
            duration: listen.track_metadata.additional_info.duration,
            origin: listen.origin(),
        };
        self.listens.push_back(row);
        if self.listens.len() > KEEP_LISTENS {
            self.listens.pop_front();
        }
    }

    pub fn latest_now_playing(&self) -> Option<&NowPlaying> {
        self.now_playing.as_ref()
    }

    pub fn list_listens(&self, limit: usize) -> Vec<ListenRow> {
        self.listens.iter().rev().take(limit).cloned().collect()
    }
}
