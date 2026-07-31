use nowplaying_proto::Listen;

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

const KEEP_LISTENS: usize = 30;

pub fn unix_now() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap_or(i64::MAX)
}

#[derive(Debug, Clone)]
pub struct NowPlaying {
    pub origin: String,
    pub origin_url: Option<String>,
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration: Option<i64>,
    pub started_at: i64,
    pub updated_at: i64,
    pub paused_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ListenRow {
    pub listened_at: i64,
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration: Option<i64>,
    pub origin: String,
    pub origin_url: Option<String>,
}

pub struct State {
    listens: VecDeque<ListenRow>,
    now_playing: Option<NowPlaying>,
}

impl State {
    pub const fn new() -> Self {
        Self {
            listens: VecDeque::new(),
            now_playing: None,
        }
    }

    pub fn set_now_playing(&mut self, listen: &Listen, now: i64) {
        let origin = listen.origin();
        let artist = listen.track_metadata.artist_name.clone();
        let title = listen.track_metadata.track_name.clone();
        let album = listen.track_metadata.release_name.clone();
        let duration = listen.track_metadata.additional_info.duration_seconds();
        let paused = listen
            .track_metadata
            .additional_info
            .paused
            .unwrap_or(false);
        let stopped = listen
            .track_metadata
            .additional_info
            .stopped
            .unwrap_or(false);
        let position = listen.track_metadata.additional_info.position;

        let same_track = self.now_playing.as_ref().is_some_and(|np| {
            np.origin == origin && np.artist == artist && np.title == title
        });

        if stopped {
            if same_track {
                self.now_playing = None;
            }
            return;
        }

        if same_track {
            let np = self.now_playing.as_mut().unwrap();
            np.origin_url
                .clone_from(&listen.track_metadata.additional_info.origin_url);
            np.album.clone_from(&listen.track_metadata.release_name);
            np.duration = listen.track_metadata.additional_info.duration_seconds();
            np.updated_at = now;
            match (position, np.paused_at.take(), paused) {
                (Some(pos), _, _) => {
                    np.started_at = now - pos.max(0);
                    np.paused_at = paused.then_some(now);
                }
                (None, Some(paused_at), false) => {
                    np.started_at += now - paused_at;
                }
                (None, _, true) => {
                    np.paused_at = Some(now);
                }
                (None, None, false) => {}
            }
        } else {
            self.now_playing = Some(NowPlaying {
                origin,
                origin_url: listen.track_metadata.additional_info.origin_url.clone(),
                artist,
                title,
                album,
                duration,
                started_at: now,
                updated_at: now,
                paused_at: paused.then_some(now),
            });
        }
    }

    pub fn insert_listen(&mut self, listen: &Listen, now: i64) {
        let row = ListenRow {
            listened_at: listen.listened_at.unwrap_or(now),
            artist: listen.track_metadata.artist_name.clone(),
            title: listen.track_metadata.track_name.clone(),
            album: listen.track_metadata.release_name.clone(),
            duration: listen.track_metadata.additional_info.duration_seconds(),
            origin: listen.origin(),
            origin_url: listen.track_metadata.additional_info.origin_url.clone(),
        };
        self.listens.push_back(row);
        if self.listens.len() > KEEP_LISTENS {
            self.listens.pop_front();
        }
    }

    pub const fn latest_now_playing(&self) -> Option<&NowPlaying> {
        self.now_playing.as_ref()
    }

    pub fn list_listens(&self, limit: usize) -> Vec<ListenRow> {
        self.listens.iter().rev().take(limit).cloned().collect()
    }
}
