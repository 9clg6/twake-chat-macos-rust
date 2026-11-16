//! Fixture catalogues and clocks shared by the view-model specs.
//! Keys: `docs/dev/ui-screens-api-sketch.md` section 1.

use twake_domain::Timestamp;
use twake_i18n::Locale;

const EN: &str = "
room-preview-you = You: { $text }
room-preview-author = { $author }: { $text }
time-yesterday = Yesterday
weekday-short-mon = Mon
weekday-short-tue = Tue
weekday-short-wed = Wed
weekday-short-thu = Thu
weekday-short-fri = Fri
weekday-short-sat = Sat
weekday-short-sun = Sun
date-short = { $month }/{ $day }/{ $year }
time-12h = { $hour }:{ $minute } { $period }
period-am = AM
period-pm = PM
day-today = Today
day-yesterday = Yesterday
new-messages = New messages
message-edited = (edited)
";

const FR: &str = "
room-preview-you = Vous : { $text }
room-preview-author = { $author } : { $text }
time-yesterday = Hier
weekday-short-mon = lun.
weekday-short-tue = mar.
weekday-short-wed = mer.
weekday-short-thu = jeu.
weekday-short-fri = ven.
weekday-short-sat = sam.
weekday-short-sun = dim.
date-short = { $day }/{ $month }/{ $year }
time-12h = { $hour }:{ $minute } { $period }
period-am = AM
period-pm = PM
day-today = Aujourd'hui
day-yesterday = Hier
new-messages = Nouveaux messages
message-edited = (modifié)
";

/// English, reader on UTC.
pub fn en() -> Locale {
    Locale::from_ftl("en", EN, 0)
}

/// French, reader on UTC.
pub fn fr() -> Locale {
    Locale::from_ftl("fr", FR, 0)
}

/// English, reader `hours` east of UTC.
pub fn en_at(hours: i32) -> Locale {
    Locale::from_ftl("en", EN, hours * 3_600)
}

/// Milliseconds since the epoch of a UTC date and time (an oracle independent of the code).
pub fn ms(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> u64 {
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = shifted.div_euclid(400);
    let year_of_era = shifted - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    ((days * 24 + hour) * 60 + minute) as u64 * 60_000
}

/// The instant of a UTC date and time.
pub fn at(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> Timestamp {
    Timestamp::from_millis(ms(year, month, day, hour, minute))
}

/// Thursday 12 November 2026, 15:30 UTC: the "now" of the specs.
pub fn now() -> Timestamp {
    at(2026, 11, 12, 15, 30)
}
