use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use mpris::{PlaybackStatus, Player, PlayerFinder};
use nowplaying_proto::{AdditionalInfo, Listen, SubmitRequest, TrackMetadata};

struct Config {
    server: String,
    token: String,
    poll_seconds: u64,
    min_play_seconds: i64,
}

impl Config {
    fn from_env() -> Result<Self> {
        let server =
            std::env::var("NOWPLAYING_SERVER").context("NOWPLAYING_SERVER must be set")?;
        let token = std::env::var("NOWPLAYING_TOKEN").context("NOWPLAYING_TOKEN must be set")?;
        Ok(Self {
            server,
            token,
            poll_seconds: env_parse("NOWPLAYING_POLL_SECONDS").unwrap_or(5),
            min_play_seconds: env_parse("NOWPLAYING_MIN_PLAY_SECONDS").unwrap_or(15),
        })
    }
}

fn env_parse<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

#[derive(Debug, Clone)]
struct PlayerInfo {
    player: String,
    status: PlaybackStatus,
    artist: String,
    title: String,
    album: Option<String>,
    length_secs: Option<i64>,
    url: Option<String>,
}

impl PlayerInfo {
    fn to_track_metadata(&self, paused: Option<bool>) -> TrackMetadata {
        TrackMetadata {
            artist_name: self.artist.clone(),
            track_name: self.title.clone(),
            release_name: self.album.clone().filter(|s| !s.is_empty()),
            additional_info: AdditionalInfo {
                duration: self.length_secs.filter(|d| *d > 0),
                duration_ms: None,
                music_service_name: Some(self.player.clone()),
                media_player: Some(self.player.clone()),
                origin_url: self.url.clone(),
                paused,
                stopped: None,
            },
        }
    }
}

fn read_player(player: &Player) -> Result<PlayerInfo> {
    let player_name = player.bus_name_trimmed().to_string();
    let status = player.get_playback_status().context("get_playback_status")?;
    let meta = player.get_metadata().unwrap_or_default();
    let artist = meta.artists().unwrap_or_default().join(", ");
    let title = meta.title().unwrap_or_default().to_string();
    let album = meta.album_name().map(str::to_string);
    let length_secs = meta
        .length()
        .map(|d| i64::try_from(d.as_micros() / 1_000_000).unwrap_or(i64::MAX));
    let url = meta.url().map(str::to_string);
    Ok(PlayerInfo {
        player: player_name,
        status,
        artist,
        title,
        album,
        length_secs,
        url,
    })
}

struct Client {
    http: reqwest::blocking::Client,
    config: Config,
}

impl Client {
    fn post(&self, req: &SubmitRequest) {
        let url = format!("{}/1/submit-listens", self.config.server);
        match self
            .http
            .post(&url)
            .header("Authorization", format!("Token {}", self.config.token))
            .json(req)
            .send()
        {
            Ok(resp) if resp.status().is_success() => {}
            Ok(resp) => eprintln!(
                "nowplaying: {} returned {} for {}",
                self.config.server,
                resp.status(),
                req.listen_type
            ),
            Err(e) => eprintln!(
                "nowplaying: failed to POST {} to {}: {e}",
                req.listen_type, self.config.server
            ),
        }
    }

    fn post_playing(&self, info: &PlayerInfo) {
        self.post_state(info, None, false);
    }

    fn post_paused(&self, info: &PlayerInfo) {
        self.post_state(info, Some(true), false);
    }

    fn post_stopped(&self, info: &PlayerInfo) {
        self.post_state(info, None, true);
    }

    fn post_state(&self, info: &PlayerInfo, paused: Option<bool>, stopped: bool) {
        let mut metadata = info.to_track_metadata(paused);
        metadata.additional_info.stopped = stopped.then_some(true);
        let req = SubmitRequest {
            listen_type: "playing_now".to_string(),
            payload: vec![Listen {
                listened_at: None,
                track_metadata: metadata,
            }],
        };
        self.post(&req);
    }

    fn post_scrobble(&self, info: &PlayerInfo, listened_at: i64) {
        let req = SubmitRequest {
            listen_type: "single".to_string(),
            payload: vec![Listen {
                listened_at: Some(listened_at),
                track_metadata: info.to_track_metadata(None),
            }],
        };
        self.post(&req);
    }
}

#[derive(Default)]
struct BridgeState {
    track: Option<PlayerInfo>,
    started_at: i64,
    segment_start: i64,
    played: i64,
}

impl BridgeState {
    fn finalize(&self, client: &Client, track: &PlayerInfo, now: i64) {
        let active = if track.status == PlaybackStatus::Playing {
            now - self.segment_start
        } else {
            0
        };
        if self.played + active >= client.config.min_play_seconds {
            client.post_scrobble(track, self.started_at);
        }
    }

    fn on_player(&mut self, client: &Client, info: &PlayerInfo, now: i64) {
        if info.status == PlaybackStatus::Stopped {
            if let Some(last) = self.track.take() {
                self.finalize(client, &last, now);
                client.post_stopped(&last);
            }
            return;
        }

        if info.title.is_empty() {
            return;
        }

        let same_track = self
            .track
            .as_ref()
            .is_some_and(|t| t.player == info.player && t.title == info.title);

        if !same_track {
            if info.status == PlaybackStatus::Paused {
                // a paused player we are not tracking must not resurrect as a
                // new now playing; end the current session instead
                if let Some(last) = self.track.take() {
                    self.finalize(client, &last, now);
                    client.post_stopped(&last);
                }
                return;
            }
            if let Some(last) = self.track.take() {
                self.finalize(client, &last, now);
            }
            client.post_playing(info);
            self.track = Some(info.clone());
            self.started_at = now;
            self.segment_start = now;
            self.played = 0;
            return;
        }

        let Some(track) = self.track.as_mut() else {
            return;
        };
        match (info.status, track.status) {
            (PlaybackStatus::Paused, PlaybackStatus::Playing) => {
                self.played += now - self.segment_start;
                self.segment_start = now;
                track.status = PlaybackStatus::Paused;
                client.post_paused(info);
            }
            (PlaybackStatus::Playing, PlaybackStatus::Paused) => {
                self.segment_start = now;
                track.status = PlaybackStatus::Playing;
                client.post_playing(info);
            }
            _ => {}
        }
    }

    fn on_no_player(&mut self, client: &Client, now: i64) {
        if let Some(track) = self.track.take() {
            self.finalize(client, &track, now);
            client.post_stopped(&track);
        }
    }

    fn tracked_player(&self) -> Option<&str> {
        self.track.as_ref().map(|t| t.player.as_str())
    }
}

fn find_player_by_bus_name(finder: &PlayerFinder, name: &str) -> Option<Player> {
    let players = finder.iter_players().ok()?;
    players
        .into_iter()
        .filter_map(Result::ok)
        .find(|p| p.bus_name_trimmed() == name)
}

fn main() -> Result<()> {
    let config = Config::from_env()?;
    let http = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(8))
        .build()
        .context("cannot build http client")?;
    let client = Client { http, config };
    let finder = PlayerFinder::new().context("cannot connect to the D-Bus session bus")?;

    let mut state = BridgeState::default();

    loop {
        std::thread::sleep(Duration::from_secs(client.config.poll_seconds));
        let now = unix_now();
        // While everything is paused, keep reporting the player we were
        // already tracking instead of flipping to an arbitrary paused one
        // (find_active prefers the alphabetically first paused player).
        let preferred = state
            .tracked_player()
            .and_then(|name| find_player_by_bus_name(&finder, name));

        let player = match finder.find_active() {
            Ok(player) => match player.get_playback_status() {
                Ok(PlaybackStatus::Paused) => preferred.or(Some(player)),
                _ => Some(player),
            },
            Err(mpris::FindingError::NoPlayerFound) => None,
            Err(e) => {
                eprintln!("nowplaying: failed to find player: {e:#}");
                continue;
            }
        };

        match player {
            Some(player) => match read_player(&player) {
                Ok(info) => state.on_player(&client, &info, now),
                Err(e) => eprintln!("nowplaying: failed to read player: {e:#}"),
            },
            None => state.on_no_player(&client, now),
        }
    }
}
