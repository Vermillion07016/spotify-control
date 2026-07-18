use std::fmt;

/// Kütüphanenin tüm public fonksiyonlarının döndürdüğü tek hata tipi.
///
/// Discord komutlarında `match` ile yakalayıp kullanıcıya anlamlı mesajlar
/// göstermek için tasarlandı (ör. `SpotifyError::NoActiveDevice` ->
/// "Önce bir cihazda Spotify açman lazım").
#[derive(Debug)]
pub enum SpotifyError {
    /// Hiç login olunmamış ya da refresh_token kaybolmuş; yeniden login gerekiyor.
    NeedsLogin,
    /// Spotify tarafında aktif/bağlı bir cihaz bulunamadı (NO_ACTIVE_DEVICE).
    NoActiveDevice,
    /// Spotify'a çok sık istek atıldı (429). `retry_after_secs` kadar beklenmeli.
    RateLimited { retry_after_secs: u64 },
    /// Gerekli bir ortam değişkeni (CLIENT_ID / CLIENT_SECRET) eksik.
    MissingEnv(String),
    /// HTTP isteği sırasında oluşan hata (bağlantı, timeout, DNS, vs.)
    Http(reqwest::Error),
    /// JSON (de)serileştirme hatası.
    Json(serde_json::Error),
    /// Dosya okuma/yazma hatası (token.json).
    Io(std::io::Error),
    /// Kategorize edilmemiş diğer hatalar.
    Other(String),
}

impl fmt::Display for SpotifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpotifyError::NeedsLogin => {
                write!(f, "Giriş yapılmamış, önce login() çağrılmalı.")
            }
            SpotifyError::NoActiveDevice => write!(
                f,
                "Aktif bir Spotify cihazı bulunamadı. Bir cihazda Spotify'ı açıp bir şarkı çalman/duraklatman gerekiyor."
            ),
            SpotifyError::RateLimited { retry_after_secs } => write!(
                f,
                "Spotify'a çok fazla istek atıldı, {} saniye sonra tekrar dene.",
                retry_after_secs
            ),
            SpotifyError::MissingEnv(name) => write!(f, "Ortam değişkeni eksik: {}", name),
            SpotifyError::Http(e) => write!(f, "HTTP hatası: {}", e),
            SpotifyError::Json(e) => write!(f, "JSON hatası: {}", e),
            SpotifyError::Io(e) => write!(f, "Dosya hatası: {}", e),
            SpotifyError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for SpotifyError {}

impl From<reqwest::Error> for SpotifyError {
    fn from(e: reqwest::Error) -> Self {
        SpotifyError::Http(e)
    }
}
impl From<serde_json::Error> for SpotifyError {
    fn from(e: serde_json::Error) -> Self {
        SpotifyError::Json(e)
    }
}
impl From<std::io::Error> for SpotifyError {
    fn from(e: std::io::Error) -> Self {
        SpotifyError::Io(e)
    }
}
impl From<std::env::VarError> for SpotifyError {
    fn from(e: std::env::VarError) -> Self {
        SpotifyError::MissingEnv(e.to_string())
    }
}

/// Başarısız bir HTTP yanıtını inceleyip anlamlı bir `SpotifyError`'a çevirir.
/// Body'yi tükettiği için sadece `!status.is_success()` durumunda çağrılmalı.
pub(crate) async fn map_error_response(res: reqwest::Response) -> SpotifyError {
    let status = res.status();

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = res
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1);
        return SpotifyError::RateLimited {
            retry_after_secs: retry_after,
        };
    }

    let text = res.text().await.unwrap_or_default();

    if status == reqwest::StatusCode::NOT_FOUND && text.contains("NO_ACTIVE_DEVICE") {
        return SpotifyError::NoActiveDevice;
    }

    SpotifyError::Other(format!("Beklenmeyen HTTP durumu {}: {}", status, text))
}