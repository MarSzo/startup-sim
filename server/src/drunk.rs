//! Drinking at work: how a drunk character talks (and types), and what the
//! breathalyser says. The level itself lives in `needs` (alcohol points).
//!
//! Slurred speech is deterministic (a seed), so the same line comes out the
//! same for everybody who hears it.

/// Alcohol points each drink adds.
pub fn alcohol_of(kind: u8) -> i32 {
    use crate::inventory::kind;
    match kind {
        kind::BEER => 15,
        kind::WINE => 30,
        kind::MALPKA => 25,
        kind::WHISKY | kind::COGNAC => 25,
        kind::VODKA => 30,
        _ => 0,
    }
}

/// The legal limit: above it the board may reprimand (0,2 ‰).
pub const LIMIT_MILLI: u32 = 200;
/// Three reprimands: fired.
pub const REPRIMANDS_TO_FIRE: u8 = 3;
/// How close the breathalyser must be held (tiles × units).
pub const BREATH_REACH: i32 = crate::sim::TILE_UNITS * 2;
/// Throwing up takes a moment (3 s); passed out: a minute (12 game minutes).
pub const VOMIT_TICKS: u32 = 3 * 20;
pub const PASS_OUT_TICKS: u32 = 60 * 20;

pub mod lines {
    pub const VOMIT: &str = "Bleeeeh… chyba jednak za dużo.";
    pub const PASS_OUT: &str = "Zzz…";
    pub const WAKE_UP: &str = "Ugh… gdzie ja jestem? Głowa mi pęka.";
    pub const NOT_BOARD: &str = "Alkomat Zarządu — to nie dla mnie.";
    pub const NOBODY: &str = "Nikogo obok — z kim to sprawdzić?";
    pub const REPRIMAND_ASK: &str = "Wynik powyżej normy. Wystawić naganę?";
    pub const REPRIMAND_YES: &str = "Wystaw naganę";
    pub const REPRIMAND_NO: &str = "Daruję tym razem";
    pub const SPARED: &str = "Tym razem bez nagany.";

    /// The reading, e.g. "Alkomat: 0,45 ‰ — Kuba jest pod wpływem!".
    pub fn reading(nick: &str, milli: u32) -> String {
        let value = format!("{},{:02}", milli / 1000, milli % 1000 / 10);
        if milli > super::LIMIT_MILLI {
            format!("Alkomat: {value} ‰ — {nick} jest pod wpływem!")
        } else if milli > 0 {
            format!("Alkomat: {value} ‰ — {nick} w normie.")
        } else {
            format!("Alkomat: {value} ‰ — {nick} trzeźwy jak szkiełko.")
        }
    }

    pub fn reprimanded(nick: &str, n: u8) -> String {
        format!("Nagana dla: {nick} (to już {n}. nagana).")
    }
}

/// A line said (or typed) at drunk tier `tier` (0 sober .. 3 very drunk):
/// drawn-out vowels, "hyyk", lisping, swapped letters - the more drunk, the
/// more. `seed` picks where (same seed, same line).
pub fn slur(text: &str, tier: u8, seed: u64) -> String {
    if tier == 0 || text.is_empty() {
        return text.to_string();
    }
    let mut rng = fastrand::Rng::with_seed(seed);
    let chance = [0, 12, 25, 40][tier.min(3) as usize];
    let mut chars: Vec<char> = Vec::with_capacity(text.len() * 2);
    for c in text.chars() {
        let lower = c.to_lowercase().next().unwrap_or(c);
        chars.push(c);
        if "aeiouyąęó".contains(lower) && rng.u32(0..100) < chance {
            for _ in 0..rng.u32(1..=tier as u32) {
                chars.push(lower); // "taaak"
            }
        } else if tier >= 2 && lower == 's' && rng.u32(0..100) < chance {
            chars.push('z'); // "szobie"
        }
    }
    // Very drunk: neighbouring letters swap places now and then.
    if tier >= 3 {
        let mut i = 1;
        while i + 1 < chars.len() {
            if chars[i].is_alphabetic() && chars[i + 1].is_alphabetic() && rng.u32(0..100) < 8 {
                chars.swap(i, i + 1);
                i += 2;
            }
            i += 1;
        }
    }
    let mut out: String = chars.into_iter().collect();
    if tier >= 2 {
        // A hiccup somewhere between words.
        let spaces: Vec<usize> = out.match_indices(' ').map(|(i, _)| i).collect();
        let hic = if tier >= 3 { " *hyyk*" } else { " *hyk*" };
        match spaces.get(rng.usize(0..spaces.len().max(1))) {
            Some(&at) => out.insert_str(at, hic),
            None => out.push_str(hic),
        }
    }
    if tier >= 3 {
        out = out.to_lowercase().replace(['.', '!'], "…");
    }
    // Keep it within what a line may carry.
    let max = crate::protocol::MAX_SAY_BYTES.min(crate::protocol::MAX_CHAT_BYTES);
    if out.len() > max {
        let mut cut = max;
        while !out.is_char_boundary(cut) {
            cut -= 1;
        }
        out.truncate(cut);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sober_speech_is_untouched_and_drunk_gets_worse() {
        let line = "Kawa gotowa! Idę na spotkanie z prezesem.";
        assert_eq!(slur(line, 0, 7), line);
        let tipsy = slur(line, 1, 7);
        let drunk = slur(line, 2, 7);
        let wasted = slur(line, 3, 7);
        assert_ne!(tipsy, line);
        assert!(drunk.contains("*hyk*") && !tipsy.contains("hyk"));
        assert!(wasted.contains("*hyyk*") && wasted == wasted.to_lowercase() && !wasted.contains('!'));
        assert!(wasted.len() > drunk.len() || wasted != drunk);
        assert_eq!(slur(line, 3, 7), wasted, "same seed, same line");
        // Long lines stay within the packet limit (and on a char boundary).
        let long = "ąę ".repeat(200);
        assert!(slur(&long, 3, 1).len() <= crate::protocol::MAX_SAY_BYTES);
    }

    #[test]
    fn readings_and_the_limit() {
        assert_eq!(lines::reading("Kuba", 450), "Alkomat: 0,45 ‰ — Kuba jest pod wpływem!");
        assert_eq!(lines::reading("Ola", 150), "Alkomat: 0,15 ‰ — Ola w normie.");
        assert!(lines::reading("Ola", 0).contains("trzeźwy"));
        assert_eq!(alcohol_of(crate::inventory::kind::BEER), 15);
        assert_eq!(alcohol_of(crate::inventory::kind::COFFEE), 0);
    }
}
