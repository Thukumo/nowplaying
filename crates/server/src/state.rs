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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayStatus {
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub struct NowPlaying {
    pub origin: String,
    pub origin_url: Option<String>,
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration: Option<i64>,
    pub status: PlayStatus,
    /// When this report was received; used to age out stale reports.
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

    /// Store the latest report as the current now playing state.
    /// The server keeps no history or extrapolation anchors of its own;
    /// freshness and position accuracy are the reporting client's job.
    pub fn report_now_playing(&mut self, listen: &Listen, now: i64) {
        if listen.track_metadata.additional_info.stopped.unwrap_or(false) {
            self.now_playing = None;
            return;
        }
        let paused = listen
            .track_metadata
            .additional_info
            .paused
            .unwrap_or(false);
        self.now_playing = Some(NowPlaying {
            origin: listen.origin(),
            origin_url: listen.track_metadata.additional_info.origin_url.clone(),
            artist: listen.track_metadata.artist_name.clone(),
            title: listen.track_metadata.track_name.clone(),
            album: listen.track_metadata.release_name.clone(),
            duration: listen.track_metadata.additional_info.duration_seconds(),
            status: if paused {
                PlayStatus::Paused
            } else {
                PlayStatus::Playing
            },
            updated_at: now,
        });
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
