use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitRequest {
    pub listen_type: String,
    pub payload: Vec<Listen>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listen {
    pub listened_at: Option<i64>,
    pub track_metadata: TrackMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMetadata {
    #[serde(default)]
    pub artist_name: String,
    #[serde(default)]
    pub track_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_name: Option<String>,
    #[serde(default)]
    pub additional_info: AdditionalInfo,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdditionalInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_player: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
}

impl AdditionalInfo {
    #[must_use]
    pub fn duration_seconds(&self) -> Option<i64> {
        self.duration
            .or_else(|| self.duration_ms.map(|ms| ms / 1000))
    }
}

impl Listen {
    #[must_use]
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
