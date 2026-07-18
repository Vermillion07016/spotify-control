use spotify_control::{add_to_queue, current_status, list_devices, login, next_track, pause, play, play_track,player_status, previous_track, queue_status, search_tracks, set_volume, SpotifyError};

/// Bir sonucu yazdırır; hata SpotifyError ise anlamlı Türkçe mesajı gösterir.
fn report<T: std::fmt::Debug>(label: &str, result: Result<T, SpotifyError>) {
    match result {
        Ok(v) => println!("[{label}] OK -> {:#?}", v),
        Err(e) => eprintln!("[{label}] HATA: {}", e),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    login().await?;

    report("player_status", player_status().await);
    report("current_status", current_status().await);
    report("list_devices", list_devices().await);
    report("queue_status", queue_status().await);

    report("pause", pause().await);
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    report("play", play().await);

    report("set_volume(50)", set_volume(50).await);

    report("next_track", next_track().await);
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    report("previous_track", previous_track().await);

    let test_track_uri = "spotify:track:4cOdK2wGLETKBW3PvgPWqT";
    report("add_to_queue", add_to_queue(test_track_uri).await);

    // --- Arama + belirli şarkıyı çalma akışı ---
    println!("\n=== search_tracks ===");
    match search_tracks("Bohemian Rhapsody", 5).await {
        Ok(tracks) => {
            for (i, t) in tracks.iter().enumerate() {
                println!(
                    "{}. {} - {} ({}) [{}]",
                    i + 1,
                    t.name,
                    t.artist_names(),
                    t.duration_formatted(),
                    t.uri
                );
            }

            if let Some(first) = tracks.first() {
                println!("\n=== play_track: {} ===", first.name);
                report("play_track", play_track(&first.uri).await);
            }
        }
        Err(e) => eprintln!("[search_tracks] HATA: {}", e),
    }

    Ok(())
}