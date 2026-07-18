# spotify-control

Spotify Web API'sini saran, Rust ile yazılmış küçük bir kütüphane (`spotify_control`).
Discord botuna gömülmek üzere tasarlandı: tek bir paylaşımlı `reqwest::Client`,
otomatik token yenileme ve Discord'a uygun hata tipleri içerir.

## Kurulum

1. [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)'da bir uygulama oluştur.
2. Uygulama ayarlarında **Redirect URI** olarak şunu ekle:
   ```
   http://127.0.0.1:8080/callback
   ```
   (Spotify, `localhost`/`127.0.0.1` loopback adresleri için `http://` kullanımına izin verir, başka adreslerde `https://` zorunludur.)
3. `.env.example` dosyasını `.env` olarak kopyala ve `CLIENT_ID` / `CLIENT_SECRET` değerlerini doldur.
4. `cargo run` ile test et — ilk çalıştırmada terminalde bir URL basılır, tarayıcıda açıp giriş yaptığında `token.json` dosyası oluşur.

## ⚠️ Sunucuya / Discord Botuna Deploy Etmek

`login()` fonksiyonu, token.json yoksa **tarayıcı tabanlı OAuth akışını** başlatır:
terminale bir URL yazdırır ve yerel `127.0.0.1:8080` portunda tıklamanı bekler.

**Bu akış, üzerinde tarayıcı olmayan bir sunucuda (VPS, Docker container, Raspberry Pi vb.) çalışmaz.**

Bu yüzden önerilen akış:

1. Botu geliştirdiğin bilgisayarda (tarayıcının olduğu yerde) `cargo run` ile bir kere
   çalıştır, tarayıcıdan girişi tamamla. Bu, yanında `refresh_token` içeren bir
   `token.json` dosyası üretir.
2. Bu `token.json` dosyasını sunucuya kopyala (botun çalışma dizinine).
3. `refresh_token` süresi dolmadığı sürece (Spotify'da pratikte süresiz, sadece
   kullanıcı erişimi manuel iptal etmediği sürece geçerli), bot bir daha
   tarayıcı akışına ihtiyaç duymadan otomatik token yenileyerek çalışmaya devam eder.
4. `token.json`'ı **asla** repoya commit etme — `.gitignore`'a ekle, içinde
   erişim/yenileme anahtarların var.

## Genel Kullanım

```rust
use spotify_control::{login, player_status, search_tracks, play_track, SpotifyError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // Uygulama başlangıcında bir kez çağır (Discord bot main'inde de öyle).
    login().await?;

    match player_status().await {
        Ok(Some(status)) => println!("Şu an çalıyor: {:?}", status.item),
        Ok(None) => println!("Aktif oynatma yok."),
        Err(SpotifyError::NoActiveDevice) => println!("Önce bir cihazda Spotify aç."),
        Err(e) => eprintln!("Hata: {e}"),
    }

    Ok(())
}
```

## Public API

| Fonksiyon | Metot | Açıklama |
|---|---|---|
| `login()` | — | İlk giriş / token.json okuma. Başlangıçta bir kez çağır. |
| `player_status()` | GET | Genel oynatıcı durumu. Aktif oynatma yoksa `Ok(None)`. |
| `current_status()` | GET | Şu an çalan şarkı. Aktif oynatma yoksa `Ok(None)`. |
| `queue_status()` | GET | Sıradaki şarkılar. |
| `list_devices()` | GET | Kullanıcının bağlı cihazları. |
| `search_tracks(query, limit)` | GET | Şarkı adına göre arama, `limit` (1-50) sonuç döner. |
| `pause()` | PUT | Duraklat. |
| `play()` | PUT | Kaldığı yerden devam ettir. |
| `play_track(uri)` | PUT | Belirli bir şarkıyı direkt çalmaya başlar. |
| `set_volume(0-100)` | PUT | Ses seviyesi (yalnızca Premium). |
| `next_track()` | POST | Sıradaki şarkı. |
| `previous_track()` | POST | Önceki şarkı. |
| `add_to_queue(uri)` | POST | Kuyruğa şarkı ekler. |

Tüm fonksiyonlar `Result<T, SpotifyError>` döner.

## Hata Tipleri (`SpotifyError`)

Discord komutlarında kullanıcıya doğru mesajı göstermek için `match` et:

```rust
match play_track(&uri).await {
    Ok(_) => { /* ... */ }
    Err(SpotifyError::NoActiveDevice) => {
        // "Önce telefon/bilgisayarında Spotify'ı açıp bir şarkı çalman lazım."
    }
    Err(SpotifyError::NeedsLogin) => {
        // "Bot Spotify'a bağlı değil, sahibiyle iletişime geç."
    }
    Err(SpotifyError::RateLimited { retry_after_secs }) => {
        // "Çok hızlı istek atıldı, {retry_after_secs} saniye sonra tekrar dene."
    }
    Err(e) => { /* genel hata, e.to_string() ile logla */ }
}
```

## Discord Komutu Taslağı (arama → seçim → çalma)

Kütüphane framework'ten (serenity/poise/twilight) bağımsız; sadece `search_tracks`
ve `play_track`'i şu şekilde bağlarsın:

```rust
// 1. Kullanıcı "/play bohemian rhapsody" yazdı
let results = search_tracks("bohemian rhapsody", 5).await?;

// 2. 5 sonucu numaralandırıp embed/select menu olarak göster
for (i, t) in results.iter().enumerate() {
    println!("{}. {} - {} ({})", i + 1, t.name, t.artist_names(), t.duration_formatted());
}

// 3. Kullanıcı bir sayı/emoji seçtiğinde (ör. "2")
let secilen = &results[1];
play_track(&secilen.uri).await?;
```

`Track` struct'ı Discord embed'i için hazır alanlar sunar: `name`, `artist_names()`,
`duration_formatted()` (mm:ss), `album.name`, `album.images` (kapak resmi URL'leri).

## Notlar / Kısıtlar

- `set_volume` ve genel oynatma kontrolü **Spotify Premium** gerektirir.
- Tüm oynatma komutları (`pause`, `play`, `next_track`, vb.) kullanıcının hesabında
  **aktif bir cihaz** ister — bir cihaz API üzerinden "uzaktan açılamaz", kullanıcının
  en az bir kere elle Spotify'ı açıp bir şey çalması/duraklatması gerekir.
- `token.json` içindeki `refresh_token` hassas veridir, güvenli sakla.