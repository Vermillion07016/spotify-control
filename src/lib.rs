use std::{sync::OnceLock, time::Duration};
use reqwest::Client;
use tokio::sync::RwLock;
use crate::{api_list::{CURRENT_STATUS, PLAYER_DEVICES, PLAYER_NEXT, PLAYER_PAUSE, PLAYER_PLAY, PLAYER_PREVIOUS, PLAYER_QUEUE, PLAYER_QUEUE_ADD, PLAYER_STATUS, PLAYER_VOLUME, SEARCH},auth::{login as login_spot, refresh_token},error::map_error_response,token::Token};

mod api_list;
mod auth;
mod models;
mod token;

pub mod error;
pub use error::SpotifyError;
pub use models::*;

static CLIENT: OnceLock<Client> = OnceLock::new();
static SAVED_TOKEN: RwLock<Option<Token>> = RwLock::const_new(None);

pub fn client() -> &'static Client {
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent("spotify-control/0.1")
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(4)
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("HTTP client oluşturulamadı")
    })
}

pub async fn login() -> Result<(), SpotifyError> {
    let token = login_spot().await?;
    *SAVED_TOKEN.write().await = Some(token);
    Ok(())
}

async fn get_valid_token() -> Result<Token, SpotifyError> {
    {
        let guard = SAVED_TOKEN.read().await;
        match guard.as_ref() {
            Some(t) if !t.is_expired() => return Ok(t.clone()),
            Some(_) => {}
            None => return Err(SpotifyError::NeedsLogin),
        }
    }

    let mut guard = SAVED_TOKEN.write().await;
    let current = guard.as_ref().ok_or(SpotifyError::NeedsLogin)?.clone();

    if !current.is_expired() {
        return Ok(current);
    }

    let refresh = current.refresh_token.clone().ok_or(SpotifyError::NeedsLogin)?;
    let refreshed = refresh_token(refresh).await?;
    *guard = Some(refreshed.clone());
    Ok(refreshed)
}

async fn active_or_first_device_id() -> Result<Option<String>, SpotifyError> {
    let devices = list_devices().await?;
    Ok(devices
        .iter()
        .find(|d| d.is_active)
        .or_else(|| devices.first())
        .and_then(|d| d.id.clone()))
}

// ---------- GET ----------

pub async fn current_status() -> Result<Option<SpotifyPlayback>, SpotifyError> {
    let token = get_valid_token().await?;

    let res = client()
        .get(CURRENT_STATUS)
        .bearer_auth(token.access_token)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }
    if res.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(None);
    }

    let text = res.text().await?;
    if text.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(serde_json::from_str::<SpotifyPlayback>(&text)?))
}

pub async fn player_status() -> Result<Option<SpotifyPlayback>, SpotifyError> {
    let token = get_valid_token().await?;

    let res = client()
        .get(PLAYER_STATUS)
        .bearer_auth(token.access_token)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }
    if res.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(None);
    }

    let text = res.text().await?;
    if text.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(serde_json::from_str::<SpotifyPlayback>(&text)?))
}

pub async fn queue_status() -> Result<SpotifyQueue, SpotifyError> {
    let token = get_valid_token().await?;

    let res = client()
        .get(PLAYER_QUEUE)
        .bearer_auth(token.access_token)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }

    Ok(res.json::<SpotifyQueue>().await?)
}

pub async fn list_devices() -> Result<Vec<Device>, SpotifyError> {
    let token = get_valid_token().await?;

    let res = client()
        .get(PLAYER_DEVICES)
        .bearer_auth(token.access_token)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }

    Ok(res.json::<SpotifyDevicesResponse>().await?.devices)
}

/// Şarkı adına göre arama yapar, en alakalı `limit` (1-50) sonucu döndürür.
///
/// `market` sabit olarak "TR" veriliyor: bu olmadan Spotify aynı şarkının
/// farklı pazarlardaki (bölgesel) kopyalarını ayrı sonuç olarak döndürebiliyor.
/// `market=from_token` da aynı sorunu kullanıcının hesap ülkesine göre çözerdi
/// ama ekstra `user-read-private` scope'u gerektiriyordu; sabit "TR" ekstra
/// scope istemeden aynı faydayı sağlıyor. Farklı bir ülke için buradan değiştir.
pub async fn search_tracks(query: &str, limit: u8) -> Result<Vec<Track>, SpotifyError> {
    let token = get_valid_token().await?;
    let limit = limit.clamp(1, 50);

    let res = client()
        .get(SEARCH)
        .bearer_auth(token.access_token)
        .query(&[
            ("q", query),
            ("type", "track"),
            ("limit", &limit.to_string()),
            ("market", "TR"),
        ])
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }

    Ok(res.json::<SearchResponse>().await?.tracks.items)
}

// ---------- PUT ----------

pub async fn pause() -> Result<(), SpotifyError> {
    let token = get_valid_token().await?;

    let res = client()
        .put(PLAYER_PAUSE)
        .bearer_auth(token.access_token)
        .body("")
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }
    Ok(())
}

/// Kaldığı yerden devam ettirir. Belirli bir şarkı çalmak için `play_track` kullan.
pub async fn play() -> Result<(), SpotifyError> {
    let token = get_valid_token().await?;
    let device_id = active_or_first_device_id().await?;

    let mut req = client().put(PLAYER_PLAY).bearer_auth(token.access_token);
    if let Some(id) = &device_id {
        req = req.query(&[("device_id", id.as_str())]);
    }

    let res = req.body("").send().await?;
    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }
    Ok(())
}

/// Belirli bir track'i (`spotify:track:ID` formatında URI) direkt çalmaya başlar.
pub async fn play_track(track_uri: &str) -> Result<(), SpotifyError> {
    let token = get_valid_token().await?;
    let device_id = active_or_first_device_id().await?;

    let mut req = client().put(PLAYER_PLAY).bearer_auth(token.access_token);
    if let Some(id) = &device_id {
        req = req.query(&[("device_id", id.as_str())]);
    }

    let body = serde_json::json!({ "uris": [track_uri] });

    let res = req.json(&body).send().await?;
    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }
    Ok(())
}

pub async fn set_volume(volume_percent: u8) -> Result<(), SpotifyError> {
    let volume_percent = volume_percent.min(100);
    let token = get_valid_token().await?;

    let res = client()
        .put(PLAYER_VOLUME)
        .bearer_auth(token.access_token)
        .query(&[("volume_percent", volume_percent.to_string())])
        .body("")
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }
    Ok(())
}

// ---------- POST ----------

pub async fn next_track() -> Result<(), SpotifyError> {
    let token = get_valid_token().await?;

    let res = client()
        .post(PLAYER_NEXT)
        .bearer_auth(token.access_token)
        .body("")
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }

    // Spotify /next ile track'i değiştirir ama ÇALMA DURUMUNU değiştirmez —
    // cihaz o an duraklatılmışsa track değişse bile duraklatılmış kalır.
    // Spotify'ın track değişikliğini backend'de işlemesi için kısa bir bekleme
    // sonrası açıkça resume ediyoruz.
    tokio::time::sleep(Duration::from_millis(250)).await;
    play().await
}

pub async fn previous_track() -> Result<(), SpotifyError> {
    let token = get_valid_token().await?;

    let res = client()
        .post(PLAYER_PREVIOUS)
        .bearer_auth(token.access_token)
        .body("")
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }

    // Aynı sebep: /previous da mevcut çalma durumunu miras alır, resume gerekiyor.
    tokio::time::sleep(Duration::from_millis(250)).await;
    play().await
}

/// `track_uri` formatı: "spotify:track:XXXXXXXXXXXXXXXXXXXXXX"
pub async fn add_to_queue(track_uri: &str) -> Result<(), SpotifyError> {
    let token = get_valid_token().await?;

    let res = client()
        .post(PLAYER_QUEUE_ADD)
        .bearer_auth(token.access_token)
        .query(&[("uri", track_uri)])
        .body("")
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }
    Ok(())
}