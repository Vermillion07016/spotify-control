use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Artist {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpotifyImage {
    pub url: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Album {
    pub name: String,
    #[serde(default)]
    pub images: Vec<SpotifyImage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub duration_ms: u64,
    #[serde(default)]
    pub artists: Vec<Artist>,
    pub album: Album,
}

impl Track {
    /// Discord'da yazdırmak için sanatçıları "Sanatçı 1, Sanatçı 2" formatına getirir.
    pub fn artist_names(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// mm:ss formatında süre.
    pub fn duration_formatted(&self) -> String {
        let total_secs = self.duration_ms / 1000;
        format!("{}:{:02}", total_secs / 60, total_secs % 60)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Device {
    pub id: Option<String>,
    pub name: String,
    pub is_active: bool,
    pub volume_percent: Option<u8>,
}

// --- API YANIT MODELLERİ ---

/// /v1/me/player | /v1/me/player/currently-playing
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpotifyPlayback {
    pub is_playing: bool,
    pub progress_ms: Option<u64>,
    pub currently_playing_type: String,
    pub item: Option<Track>,

    pub device: Option<Device>,
    pub shuffle_state: Option<bool>,
    pub repeat_state: Option<String>,
}

/// /v1/me/player/queue
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpotifyQueue {
    pub currently_playing: Option<Track>,
    #[serde(default)]
    pub queue: Vec<Track>,
}

/// /v1/me/player/devices
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpotifyDevicesResponse {
    pub devices: Vec<Device>,
}

/// /v1/search?type=track
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResponse {
    pub tracks: SearchTracksPage,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchTracksPage {
    #[serde(default)]
    pub items: Vec<Track>,
}