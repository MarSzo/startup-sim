//! Coffee cups and the cleaner (backlog): a drunk (or gone cold) coffee
//! leaves an empty mug in your hands. Leave it anywhere (drop it), wash it at
//! a sink or take it back to the coffee machine for a refill. At the end of
//! the afternoon (15:00-16:00) the cleaner does her round: walks to every mug left lying
//! around, all floors, and collects them - grumbling about rooms full of
//! mugs, and when the day's haul is big, on #ogólny too (naming the record
//! holder).

/// The round starts at a random minute between 15:00 and 16:00.
pub const ROUND_AT: u32 = 15 * 60;
pub const ROUND_SPREAD: u32 = 60;
/// This many mugs in one room = a grumble on the spot.
pub const ROOM_COMPLAINT: u32 = 3;
/// This many in a day = a post on #ogólny.
pub const DAY_COMPLAINT: u32 = 5;
/// She stays a moment where she picked mugs up (wiping): 2 s.
pub const WIPE_TICKS: u32 = 40;

/// "1 kubek", "3 kubki", "5 kubków".
pub fn mugs(n: u32) -> String {
    let word = if n == 1 {
        "kubek"
    } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
        "kubki"
    } else {
        "kubków"
    };
    format!("{n} {word}")
}

pub mod lines {
    use super::mugs;
    pub const HELLO: &str = "Dzień dobry! Kubki proszę odnosić — do ekspresu albo do umywalki, dobrze?";
    pub const BUSY: &str = "Sprzątam, sprzątam — uwaga, mokra podłoga!";
    pub const START: &str = "Dzień dobry, sprzątanie! Zaczynam obchód.";
    pub const SPOTLESS: &str = "Czysto dziś, aż miło! Tak trzymać.";
    pub fn room_mess(n: u32) -> String {
        format!("No nie… {} w jednym pokoju! To jakaś kolekcja?", mugs(n))
    }
    pub fn few(n: u32) -> String {
        format!("{} zebrane. Da się żyć.", capitalize(&mugs(n)))
    }
    pub fn done_many(n: u32) -> String {
        format!("Uff, {} dzisiaj! Ręce mi odpadają.", mugs(n))
    }
    /// The #ogólny post; `record`: (nick, mugs) of the worst offender.
    pub fn post(n: u32, record: Option<(&str, u32)>) -> String {
        let mut s = format!(
            "Kochani, dziś zebrałam z biura {} po kawie. Pusty kubek można umyć przy umywalce albo odnieść do ekspresu!",
            mugs(n)
        );
        if let Some((nick, k)) = record {
            s.push_str(&format!(" Rekordzista dnia: {nick} ({}).", mugs(k)));
        }
        s.push_str(" — Pani Krysia");
        s
    }
    pub const WASHED: &str = "Kubek umyty, stoi na suszarce.";
    pub const REFILL: &str = "Dolewka do tego samego kubka.";
    fn capitalize(s: &str) -> String {
        let mut c = s.chars();
        c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polish_plurals_of_mugs() {
        assert_eq!(mugs(1), "1 kubek");
        assert_eq!(mugs(3), "3 kubki");
        assert_eq!(mugs(5), "5 kubków");
        assert_eq!(mugs(12), "12 kubków");
        assert_eq!(mugs(22), "22 kubki");
        assert!(lines::post(7, Some(("Bob", 4))).contains("Rekordzista dnia: Bob (4 kubki)"));
    }
}
