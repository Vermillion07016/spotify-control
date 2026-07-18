use std::{collections::HashMap, fs, time::SystemTime};

use reqwest::Url;
use tiny_http::{Request, Response, Server};

use crate::{
    api_list::{AUTHORIZE, CALLBACKURL, TOKENAPI},
    client,
    error::{map_error_response, SpotifyError},
    token::{Token, TokenError},
};

/// Yerel 8080 portunda tek seferlik bir HTTP sunucusu açıp Spotify'ın
/// yönlendirdiği `?code=...` parametresini yakalar. Bu fonksiyon senkron ve
/// bloklayan bir soket kabul işlemi yapar; async context'ten çağırırken
/// mutlaka `tokio::task::spawn_blocking` ile sarmalanmalı, aksi halde
/// tokio runtime thread'ini kilitler.
fn start_callback_listener() -> Option<String> {
    let server = match Server::http("127.0.0.1:8080") {
        Ok(c) => c,
        Err(_) => {
            eprintln!(
                "Sunucu 8080 portuna bind edilemedi. Lütfen 8080 portu kullanan programı kapatınız."
            );
            return None;
        }
    };

    let request = server.incoming_requests().next()?;
    let url = Url::parse(&format!("http://127.0.0.1:8080{}", request.url())).ok()?;

    if let Some((_, code)) = url.query_pairs().find(|(k, _)| k == "code") {
        send_response(
            request,
            "Giriş başarılı! Botu artık kullanabilirsiniz. Bu sekmeyi kapatabilirsiniz.",
        );
        return Some(code.to_string());
    }
    send_response(request, "Giriş başarısız. Botun sahibi ile konuşun");
    None
}

fn send_response(request: Request, response: &str) {
    let response = Response::from_string(response);
    if let Err(why) = request.respond(response) {
        eprintln!("Error while sending http response: {}", why);
    }
}

async fn get_token() -> Result<Token, SpotifyError> {
    println!("Getting new token");

    let client_id = std::env::var("CLIENT_ID")?;
    let client_secret = std::env::var("CLIENT_SECRET")?;

    let mut base_url =
        Url::parse(AUTHORIZE).map_err(|e| SpotifyError::Other(e.to_string()))?;
    base_url
        .query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", CALLBACKURL)
        .append_pair(
            "scope",
            "user-modify-playback-state user-read-playback-state user-read-currently-playing",
        );

    println!("click this url > {}", base_url.as_str());

    // Bloklayan sunucuyu ayrı bir thread'de çalıştır, tokio runtime'ı bekletme.
    let code = tokio::task::spawn_blocking(start_callback_listener)
        .await
        .map_err(|e| SpotifyError::Other(format!("Callback thread hatası: {}", e)))?
        .ok_or_else(|| SpotifyError::Other("Auth kodunu alırken bir sorun yaşandı".into()))?;

    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("code", code.as_str());
    params.insert("redirect_uri", CALLBACKURL);

    let res = client()
        .post(TOKENAPI)
        .basic_auth(client_id, Some(client_secret))
        .form(&params)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }

    let mut token = res.json::<Token>().await?;
    save_token(&mut token)?;
    Ok(token)
}

pub async fn refresh_token(refresh_token: String) -> Result<Token, SpotifyError> {
    let client_id = std::env::var("CLIENT_ID")?;
    let client_secret = std::env::var("CLIENT_SECRET")?;

    let mut params = HashMap::new();
    params.insert("refresh_token", refresh_token.as_str());
    params.insert("grant_type", "refresh_token");

    let res = client()
        .post(TOKENAPI)
        .basic_auth(client_id, Some(client_secret))
        .form(&params)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(map_error_response(res).await);
    }

    let mut token = res.json::<Token>().await?;

    // Spotify yenileme yanıtında çoğunlukla yeni bir refresh_token döndürmez;
    // döndürmediyse eskisini kaybetmemek için koru.
    if token.refresh_token.is_none() {
        token.refresh_token = Some(refresh_token);
    }

    save_token(&mut token)?;
    Ok(token)
}

fn save_token(t: &mut Token) -> Result<(), SpotifyError> {
    t.obtained_at = Some(SystemTime::now());
    let content = serde_json::to_string(t)?;
    fs::write("token.json", content)?;
    Ok(())
}

/// token.json varsa okur, yoksa/bozuksa tarayıcı tabanlı OAuth akışını başlatır.
/// Süresi dolmuşsa otomatik olarak yeniler.
///
/// ÖNEMLİ: `get_token()` bir tarayıcıda açılıp tıklanması gereken bir URL
/// yazdırır ve yerel 8080 portunda bağlantı bekler. Bu, üzerinde tarayıcı
/// olmayan bir sunucuda (ör. Discord botunun çalıştığı VPS) headless olarak
/// ÇALIŞMAZ. Bkz. README.md → "Sunucuya Deploy Etmek".
pub async fn login() -> Result<Token, SpotifyError> {
    let mut token = match Token::read_from_file() {
        Err(why) => {
            match why {
                TokenError::FileNotFound(why) => {
                    eprintln!("token.json dosyası bulunamadı: {} | yeni token alınıyor.", why)
                }
                TokenError::FileReadError(why) => {
                    eprintln!("token.json dosyası bozuk: {} | yeni token alınıyor.", why)
                }
                TokenError::InvalidTokenSaveFile(why) => {
                    eprintln!("token.json dosyası bozuk: {} | yeni token alınıyor.", why)
                }
            }
            get_token().await?
        }
        Ok(t) => t,
    };

    if token.is_expired() {
        let refresh = token
            .refresh_token
            .clone()
            .ok_or(SpotifyError::NeedsLogin)?;
        token = refresh_token(refresh).await?;
    }

    Ok(token)
}