//! The chill room's TV (channels drawn by the client, picked with the
//! remote) and the boombox (tracks, played where it is). Everybody sees
//! and hears the same moment: the server says when it started.

/// TV channels (`Media` channel = index + 1; 0 = off).
pub const CHANNELS: [&str; 5] = ["Kreskówki", "Wiadomości", "Pogoda", "Mecz", "Przyroda"];
/// Boombox tracks (track = index + 1; 0 = off); the client has them as
/// `boombox_<n>`.
pub const TRACKS: [&str; 4] = ["Disco polo na full", "Lo-fi do kodowania", "Techno z piwnicy", "Szanty z biura"];
/// The remote works this close to the screen (8 tiles, the same floor).
pub const REMOTE_REACH: i32 = crate::sim::TILE_UNITS * 8;
/// Dialog ids: the TV's channels, the boombox's tracks.
pub const TV_DIALOG: u8 = 253;
pub const BOOMBOX_DIALOG: u8 = 254;
/// `Media` is sent this often (1 s) and on every change.
pub const SEND_TICKS: u32 = 20;

pub mod lines {
    pub const TV_ASK: &str = "Co oglądamy?";
    pub const MUSIC_ASK: &str = "Co puszczamy?";
    pub const OFF: &str = "Wyłącz";
    pub const TV_OFF: &str = "Telewizor wyłączony. Wracamy do pracy?";
    pub const MUSIC_OFF: &str = "Cisza. Ktoś w końcu odetchnie.";
    pub const NOT_HERE: &str = "Pilot działa tylko przy telewizorze w chill roomie.";
    pub fn tv_on(name: &str) -> String {
        format!("Przełączam na: {name}")
    }
    pub fn music_on(name: &str) -> String {
        format!("Puszczam: {name}! 🎶")
    }
}
