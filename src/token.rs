use serde::{Deserialize, Serialize};
use std::{fs::File,io::{BufReader, Read},time::SystemTime};

/// Bir isteğin gönderilip Spotify'a ulaşana kadar geçen süreyi tolere etmek
/// için token'ı gerçek süresinden bu kadar saniye erken "süresi dolmuş" say.
const EXPIRY_MARGIN_SECS: u64 = 60;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u16,
    pub refresh_token: Option<String>,
    pub scope: String,
    pub obtained_at: Option<SystemTime>,
}

pub enum TokenError {
    FileReadError(String),
    FileNotFound(String),
    InvalidTokenSaveFile(String),
}

impl Token {
    pub fn read_from_file() -> Result<Self, TokenError> {
        let file =
            File::open("token.json").map_err(|err| TokenError::FileNotFound(err.to_string()))?;
        let mut reader = BufReader::new(file);
        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .map_err(|err| TokenError::FileReadError(err.to_string()))?;
        let token = serde_json::from_str(&content)
            .map_err(|err| TokenError::InvalidTokenSaveFile(err.to_string()))?;

        Ok(token)
    }

    /// Token'ın süresinin dolup dolmadığını, güvenlik payı ile birlikte kontrol eder.
    pub fn is_expired(&self) -> bool {
        let obtained_at = self.obtained_at.unwrap_or(std::time::UNIX_EPOCH);
        std::time::SystemTime::now()
            .duration_since(obtained_at)
            .map(|elapsed| elapsed.as_secs() + EXPIRY_MARGIN_SECS > self.expires_in as u64)
            // Sistem saati geride ise (obtained_at gelecekte kalıyorsa) güvenli
            // tarafta kal ve süresi dolmuş say.
            .unwrap_or(true)
    }
}