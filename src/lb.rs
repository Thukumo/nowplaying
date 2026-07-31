use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SubmitRequest {
    pub listen_type: String,
    pub payload: Vec<Listen>,
}

#[derive(Debug, Deserialize)]
pub struct Listen {
    pub listened_at: Option<i64>,
    pub track_metadata: TrackMetadata,
}

#[derive(Debug, Deserialize)]
pub struct TrackMetadata {
    #[serde(default)]
    pub artist_name: String,
    #[serde(default)]
    pub track_name: String,
    pub release_name: Option<String>,
    #[serde(default)]
    pub additional_info: AdditionalInfo,
}

#[derive(Debug, Deserialize, Default)]
pub struct AdditionalInfo {
    pub duration: Option<i64>,
    pub duration_ms: Option<i64>,
    pub music_service_name: Option<String>,
    pub media_player: Option<String>,
    pub origin_url: Option<String>,
    pub paused: Option<bool>,
    pub stopped: Option<bool>,
    pub position: Option<i64>,
}

impl AdditionalInfo {
    pub fn duration_seconds(&self) -> Option<i64> {
        self.duration
            .or_else(|| self.duration_ms.map(|ms| ms / 1000))
    }
}

impl Listen {
    pub fn origin(&self) -> String {
        self.track_metadata
            .additional_info
            .music_service_name
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| self.track_metadata.additional_info.media_player.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string())
    }
}
