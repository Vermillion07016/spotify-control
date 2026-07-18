pub const CALLBACKURL: &str = "http://127.0.0.1:8080/callback";

pub const AUTHORIZE: &str = "https://accounts.spotify.com/authorize";
pub const TOKENAPI: &str = "https://accounts.spotify.com/api/token";

const PLAYER_BASE: &str = "https://api.spotify.com/v1/me/player";

// GET
pub const PLAYER_STATUS: &str = PLAYER_BASE;
pub const CURRENT_STATUS: &str = "https://api.spotify.com/v1/me/player/currently-playing";
pub const PLAYER_QUEUE: &str = "https://api.spotify.com/v1/me/player/queue";
pub const PLAYER_DEVICES: &str = "https://api.spotify.com/v1/me/player/devices";
pub const SEARCH: &str = "https://api.spotify.com/v1/search";

// PUT
pub const PLAYER_PAUSE: &str = "https://api.spotify.com/v1/me/player/pause";
pub const PLAYER_PLAY: &str = "https://api.spotify.com/v1/me/player/play";
pub const PLAYER_VOLUME: &str = "https://api.spotify.com/v1/me/player/volume";

// POST
pub const PLAYER_NEXT: &str = "https://api.spotify.com/v1/me/player/next";
pub const PLAYER_PREVIOUS: &str = "https://api.spotify.com/v1/me/player/previous";
pub const PLAYER_QUEUE_ADD: &str = "https://api.spotify.com/v1/me/player/queue";