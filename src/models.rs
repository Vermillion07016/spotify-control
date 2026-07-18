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
    /// "YYYY", "YYYY-MM" ya da "YYYY-MM-DD" formatında olabilir (Spotify'ın
    /// `release_date_precision` alanına göre değişir).
    pub release_date: Option<String>,
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
    /// 0-100 arası; sadece arama/full track objelerinde gelir. Aynı isimli
    /// birden fazla sürüm arasında hangisinin "asıl"/en bilinen olduğunu
    /// ayırt etmeye yardımcı olur (yüksek = daha popüler).
    #[serde(default)]
    pub popularity: Option<u8>,
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

    /// Albüm adından sadece yılı çıkarır (varsa), yoksa "?" döner.
    pub fn release_year(&self) -> &str {
        self.album
            .release_date
            .as_deref()
            .and_then(|d| d.get(0..4))
            .unwrap_or("?")
    }

    /// Discord'da arama sonucu listesinde göstermek için tek satırlık,
    /// aynı isimli şarkıları birbirinden ayırt etmeye yardımcı bir etiket.
    /// Örn: "Bohemian Rhapsody - Queen · A Night at the Opera (1975) · 4:59"
    pub fn search_result_label(&self) -> String {
        format!(
            "{} - {} · {} ({}) · {}",
            self.name,
            self.artist_names(),
            self.album.name,
            self.release_year(),
            self.duration_formatted()
        )
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