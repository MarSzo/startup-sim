//! Pay ranges, expectations and the contract: what the job ad promises,
//! what you asked for, and what HR puts on paper (a bit less, surprise).

use crate::protocol::employment;

/// Working hours in a month: zł a month -> grosze an hour.
pub const HOURS_A_MONTH: i64 = 168;
/// Positions added in the game: this range until the founder changes it.
pub const DEFAULT_RANGE: [u32; 2] = [6000, 9000];
/// Expected pay accepted by the form (zł a month).
pub const SALARY_MIN: u32 = 1000;
pub const SALARY_MAX: u32 = 100_000;
/// HR's "small correction": the contract says this much less (%).
pub const CUT_PERCENT: std::ops::RangeInclusive<u32> = 10..=25;
/// B2B: more on paper (no employer's costs), but no advance.
pub const B2B_BONUS_PERCENT: u32 = 20;
/// A contract of mandate: students only, under this age.
pub const MANDATE_AGE: u8 = 26;
/// The contract dialog (after the R menu's 250 and the cupboard's 251).
pub const CONTRACT_ID: u8 = 252;

/// What was agreed in the application, and what HR offers (0 = not shown yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Terms {
    pub agreed: u32,
    pub form: u8,
    pub offered: u32,
}

/// A form of employment this candidate may choose.
pub fn form_allowed(form: u8, student: bool, age: u8) -> bool {
    match form {
        employment::EMPLOYMENT | employment::B2B => true,
        employment::MANDATE => student && age < MANDATE_AGE,
        _ => false,
    }
}

/// The amount on the contract: `cut` % less than agreed (B2B: +20% on top),
/// rounded down to 100 zł.
pub fn offered(agreed: u32, form: u8, cut: u32) -> u32 {
    let mut v = u64::from(agreed) * u64::from(100 - cut.min(100)) / 100;
    if form == employment::B2B {
        v = v * u64::from(100 + B2B_BONUS_PERCENT) / 100;
    }
    (v / 100 * 100) as u32
}

/// Grosze per game hour for a monthly salary.
pub fn hourly(monthly: u32) -> i64 {
    i64::from(monthly) * 100 / HOURS_A_MONTH
}

pub fn form_name(form: u8) -> &'static str {
    match form {
        employment::B2B => "B2B",
        employment::MANDATE => "umowa zlecenie",
        _ => "umowa o pracę",
    }
}

/// "8 500 zł".
pub fn zl(v: u32) -> String {
    let s = v.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('\u{a0}');
        }
        out.push(c);
    }
    format!("{out} zł")
}

pub mod lines {
    use super::{form_name, zl};

    pub fn contract(title: &str, form: u8, offered: u32, agreed: u32) -> String {
        format!(
            "Umowa: {title}, {}. Wynagrodzenie: {} brutto miesięcznie. (Na rozmowie było {}… „Drobna korekta, standard w branży.”)",
            form_name(form),
            zl(offered),
            zl(agreed)
        )
    }
    pub const SIGN: &str = "Podpisuję";
    pub const RESIGN: &str = "Rezygnuję";
    pub fn signed(dept: Option<&str>, advance: bool) -> String {
        let welcome = dept.map_or_else(|| "witamy w firmie!".to_string(), |d| format!("witamy w dziale {d}!"));
        if advance {
            format!("Umowa podpisana — {welcome} Oto karta pracownika, Twój laptop i 200 zł zaliczki na start.")
        } else {
            format!("Umowa podpisana — {welcome} Oto karta i laptop. Zaliczek u nas nie ma — rozliczasz się sama/sam.")
        }
    }
    pub const RESIGNED: &str = "Szkoda. Odprowadzę na portiernię — przepustkę trzeba oddać.";
    pub const SEE_OUT_GAVE_UP: &str = "Nie czekam dłużej — przepustkę i tak trzeba oddać na portierni.";
    pub const PASS_BACK: &str = "Przepustkę poproszę. Dziękuję i do widzenia — powodzenia gdzie indziej!";
    pub const TOO_MUCH_SUBJECT: &str = "Odpowiedź na zgłoszenie";
    pub fn too_much(nick: &str, title: &str, max: u32) -> String {
        format!(
            "Cześć {nick}!\n\nDziękujemy za zgłoszenie na stanowisko {title}. Niestety Twoje oczekiwania finansowe przekraczają budżet \
             (maksymalnie {}). Trzymamy kciuki!\n\nZespół rekrutacji",
            zl(max)
        )
    }
    pub fn resigned_mail(nick: &str) -> String {
        format!("Cześć {nick}!\n\nSzkoda, że się nie dogadaliśmy. Twoje zgłoszenie zostaje w naszej bazie (obiecujemy).\n\nHR")
    }
    pub const LUNCH: &str = "Dzień dobry! Zamówił/a już Pan/Pani obiad? Zamawia się w aplikacji na komputerze, do 13:00!";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contracts_come_out_lower_b2b_higher_and_mandates_are_for_students() {
        assert_eq!(offered(10_000, employment::EMPLOYMENT, 15), 8500);
        assert_eq!(offered(10_000, employment::B2B, 15), 10_200);
        assert_eq!(offered(9_999, employment::EMPLOYMENT, 10), 8_900, "rounded down to 100 zł");
        assert!(form_allowed(employment::MANDATE, true, 25));
        assert!(!form_allowed(employment::MANDATE, true, 26) && !form_allowed(employment::MANDATE, false, 20));
        assert!(form_allowed(employment::B2B, false, 40) && !form_allowed(9, true, 20));
        assert_eq!(hourly(8400), 5000, "8 400 zł a month = 50 zł an hour");
        assert_eq!(zl(12_500), "12\u{a0}500 zł");
        assert_eq!(zl(900), "900 zł");
    }
}
