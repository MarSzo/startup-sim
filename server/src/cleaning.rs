//! Coffee cups and the cleaner (backlog): a drunk (or gone cold) coffee
//! leaves an empty mug in your hands. Leave it anywhere (drop it), wash it at
//! a sink or take it back to the coffee machine for a refill. At the end of
//! the afternoon (15:00-16:00) the cleaner does her round: walks to every mug left lying
//! around, all floors, and collects them - grumbling about rooms full of
//! mugs, and when the day's haul is big, on #ogólny too (naming the record
//! holder). On the way she mops up the puddles left by toilet accidents.

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
    /// Mopping up an accident puddle.
    pub const PUDDLE: &str = "Co za cham tu naszczał!";
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
        let mut s =
            format!("Kochani, dziś zebrałam z biura {} po kawie. Pusty kubek można umyć przy umywalce albo odnieść do ekspresu!", mugs(n));
        if let Some((nick, k)) = record {
            s.push_str(&format!(" Rekordzista dnia: {nick} ({}).", mugs(k)));
        }
        s.push_str(" — Pani Maria");
        s
    }
    /// Pani Maria talks. All the time. To everyone. About everything.
    pub const STORIES: [&str; 30] = [
        "Wie Pan/Pani, że u nas na osiedlu znowu rozkopali ulicę? Trzeci raz w tym roku!",
        "Mój Zbyszek wczoraj naprawiał kran. Teraz mamy fontannę w łazience.",
        "Na rynku pomidory po dwanaście złotych! Za moich czasów to za tyle był cały obiad.",
        "Moja synowa robi te całe… jak to się nazywa… influencerstwo. I z tego żyje! No nie wiem.",
        "Słyszał Pan/Pani, że zamykają tę piekarnię na rogu? Teraz będzie tam bank. Czwarty na ulicy.",
        "Wnuczek mi pokazywał tego czata, co wszystko wie. Zapytałam o przepis na bigos — dobry był!",
        "Tramwaj dziś znowu się spóźnił. Motorniczy pewnie też miał kawę w kubku po kimś.",
        "U sąsiadki kot uciekł na dach. Straż przyjechała! Kot zszedł sam, jak tylko ich zobaczył.",
        "Ja to w tym budynku sprzątam od samego początku. Jak tu jeszcze była hurtownia skarpet.",
        "Na działce mi w tym roku cukinie urosły jak bejsbole. Przynieść Panu/Pani jedną?",
        "Mówią, że w czwartek ma być burza. Moje kolano mówi, że już w środę.",
        "Mój Zbyszek kupił sobie hulajnogę elektryczną. W wieku sześćdziesięciu trzech lat! Szaleniec.",
        "A ten prezes to taki miły człowiek, tylko kubków po sobie nie odnosi. Jak wszyscy.",
        "W sobotę byłam na weselu chrześnicy. Do czwartej rano tańczyłam! Nogi do dziś mnie bolą.",
        "Wie Pan/Pani, ile teraz kosztuje mop? Sto dwadzieścia złotych! Złoty ten mop czy co?",
        "Na Facebooku pisali, że w parku widzieli dzika. Ja tam wieczorem już nie chodzę.",
        "Moja córka mówi, żebym poszła na emeryturę. A kto by wam tu kubki zbierał, co?",
        "Wczoraj w telewizji był taki serial… ona go kochała, a on był bliźniakiem! Nie do wiary.",
        "Pani Paulina znowu siedzi w tym fotelu? Ona tak od rana. Ja to bym nie umiała.",
        "W mieście otworzyli nową kawiarnię. Kawa za dwadzieścia osiem złotych! W kubku z papieru!",
        "Mój wnuk też jest programistą. Naprawia komputery. A może nie naprawia, nie wiem dokładnie.",
        "Proszę uważać, w kuchni mokra podłoga. No, teraz już sucha. Ale była mokra!",
        "Sąsiad z góry znowu wiercił w niedzielę. Siedem lat ten remont. Siedem!",
        "Wie Pan/Pani, że pierogi najlepsze są odsmażane na drugi dzień? To naukowo udowodnione.",
        "W autobusie dzisiaj jeden pan jadł śledzia. O siódmej rano! Cały autobus pachniał.",
        "Mój Zbyszek mówi, że te całe startupy to bańka. Ale on mówił to samo o internecie.",
        "Kupiłam sobie smartwatch. Liczy mi kroki. Dziś już dwanaście tysięcy, a jeszcze nie skończyłam!",
        "Na targu spotkałam koleżankę ze szkoły. Ma już prawnuka! A ja dopiero wnuka. Ech.",
        "Ten nowy z drugiego piętra to taki grzeczny. Zawsze mówi dzień dobry. Nie to co niektórzy.",
        "No dobrze, nie przeszkadzam. Ale jeszcze tylko jedno powiem…",
    ];

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
