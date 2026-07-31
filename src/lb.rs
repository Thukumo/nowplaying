use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SubmitRequest {
    #[serde(rename = "listen_type")]
    pub listen_type: String,
    pub payload: Vec<Listen>,
}

#[derive(Debug, Deserialize)]
pub struct Listen {
    #[serde(default)]
    pub listened_at: Option<i64>,
    #[serde(default)]
    pub playing: Option<bool>,
    #[serde(rename = "track_metadata")]
    pub track_metadata: TrackMetadata,
}

#[derive(Debug, Deserialize)]
pub struct TrackMetadata {
    #[serde(rename = "artist_name", default)]
    pub artist_name: Artist,
    #[serde(rename = "track_name", default)]
    pub track_name: String,
    #[serde(rename = "release_name", default)]
    pub release_name: Option<String>,
    #[serde(rename = "additional_info", default)]
    pub additional_info: AdditionalInfo,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Artist {
    Single(String),
    Multi(Vec<String>),
}

impl Default for Artist {
    fn default() -> Self {
        Self::Single(String::new())
    }
}

impl Artist {
    pub fn as_string(&self) -> String {
        match self {
            Self::Single(s) => s.clone(),
            Self::Multi(v) => v.join(" & "),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct AdditionalInfo {
    #[serde(default)]
    pub duration: Option<i64>,
    #[serde(rename = "music_service_name", default)]
    pub music_service_name: Option<String>,
}

impl Listen {
    pub fn origin(&self) -> String {
        self.track_metadata
            .additional_info
            .music_service_name
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn is_playing(&self) -> bool {
        self.playing.unwrap_or(true)
    }
}
