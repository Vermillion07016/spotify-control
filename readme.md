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

İstenen izinler (scope): `user-modify-playback-state user-read-playback-state user-read-currently-playing`.
Scope listesi ileride değişirse **`token.json`'ı silip yeniden login olman gerekir** —
`refresh_token` mevcut scope'larla sınırlıdır, yeni scope kazandırmaz.

> **Not (arama/market):** `search_tracks`, sonuçları tek bir pazara sabitlemek için
> `market=TR` gönderiyor (aksi halde Spotify aynı şarkının farklı bölgesel
> kopyalarını ayrı sonuç gibi döndürebiliyor). Farklı bir ülke için
> `src/lib.rs` içindeki `search_tracks` fonksiyonundaki `"TR"` değerini değiştir.

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
use spotify_control::{login, player_status, search_tracks, play_track_preserving_queue, SpotifyError};

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

    // Arama yapıp ilk sonucu, kuyruğu bozmadan çal.
    if let Some(track) = search_tracks("bohemian rhapsody", 5).await?.into_iter().next() {
        play_track_preserving_queue(&track.uri).await?;
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
| `play_track(uri)` | PUT | Belirli bir şarkıyı direkt çalmaya başlar. ⚠️ Kullanıcının albüm/playlist context'ini/kuyruğunu tamamen değiştirir, bkz. aşağıdaki not. |
| `play_track_preserving_queue(uri)` | PUT+POST | Şarkıyı kuyruğa ekleyip oraya atlar; context/kuyruk **bozulmaz**. Discord'da "arama sonucundan çal" komutu için önerilen fonksiyon budur. |
| `set_volume(0-100)` | PUT | Ses seviyesi (yalnızca Premium). |
| `next_track()` | POST | Sıradaki şarkı. Track değiştirdikten sonra otomatik olarak resume da eder. |
| `previous_track()` | POST | Önceki şarkı. Track değiştirdikten sonra otomatik olarak resume da eder. |
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
ve `play_track_preserving_queue`'yu şu şekilde bağlarsın:

```rust
// 1. Kullanıcı "/play bohemian rhapsody" yazdı
let results = search_tracks("bohemian rhapsody", 5).await?;

// 2. 5 sonucu numaralandırıp embed/select menu olarak göster.
// search_result_label(), aynı isimli farklı sürümleri (remaster, canlı, vs.)
// albüm adı + yıl + süre ile ayırt etmeni sağlar.
for (i, t) in results.iter().enumerate() {
    println!("{}. {}", i + 1, t.search_result_label());
}

// 3. Kullanıcı bir sayı/emoji seçtiğinde (ör. "2")
let secilen = &results[1];
play_track_preserving_queue(&secilen.uri).await?;
```

`Track` struct'ı Discord embed'i için hazır alanlar sunar: `name`, `artist_names()`,
`duration_formatted()` (mm:ss), `search_result_label()` (arama sonucu için tek satır
özet), `album.name`, `album.release_date`, `album.images` (kapak resmi URL'leri),
`popularity` (0-100).

### `play_track` yerine neden `play_track_preserving_queue`?

`play_track(uri)`, Spotify'a context olarak sadece `{"uris": [uri]}` gönderir.
Bu, kullanıcının o an dinlediği albümü/playlist'i/kuyruğu **tamamen değiştirip**
yerine tek şarkılık geçici bir oturum koyar. Sonrasında `next_track()` /
`previous_track()` artık kullanıcının asıl kuyruğunu değil, bu tek şarkılık
context'i görür — `previous` hep aynı şarkıya döner, `next`'in gidecek yeri
kalmaz.

`play_track_preserving_queue(uri)` bunun yerine şarkıyı mevcut kuyruğun
**sonuna ekleyip** oraya `next_track()` ile atlar. Context asla değişmediği
için sonraki `next`/`previous` komutları normal çalışmaya devam eder. Discord
botunda arama sonucundan şarkı çaldırırken bunu kullan; `play_track`'i yalnızca
bilerek context'i tamamen sıfırlamak istediğin nadir durumlar için sakla.

## Notlar / Kısıtlar

- `set_volume` ve genel oynatma kontrolü **Spotify Premium** gerektirir.
- Tüm oynatma komutları (`pause`, `play`, `next_track`, vb.) kullanıcının hesabında
  **aktif bir cihaz** ister — bir cihaz API üzerinden "uzaktan açılamaz", kullanıcının
  en az bir kere elle Spotify'ı açıp bir şey çalması/duraklatması gerekir.
- `token.json` içindeki `refresh_token` hassas veridir, güvenli sakla.