//! Calendar and clock arithmetic of the reader, shared by the row, the divider and the bubble.

use twake_domain::{Day, Timestamp};
use twake_i18n::{Arg, HourCycle, Locale};

const SECS_PER_DAY: i64 = 86_400;

/// Days since 1970-01-01 of a calendar day (Howard Hinnant's `days_from_civil`).
pub fn day_number(day: Day) -> i64 {
    let year = i64::from(day.year) - i64::from(day.month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month = i64::from(day.month);
    let shifted = (month + 9) % 12;
    let day_of_year = (153 * shifted + 2) / 5 + i64::from(day.day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The day of the reader that `ts` falls in, as a day number.
pub fn local_day(ts: Timestamp, locale: &Locale) -> i64 {
    day_number(Day::of(ts, locale.utc_offset_secs()))
}

/// `14:05`, or `2:05 PM` on a 12 hour clock.
pub fn clock_text(ts: Timestamp, locale: &Locale) -> String {
    let secs = (ts.as_millis() / 1_000) as i64 + i64::from(locale.utc_offset_secs());
    let of_day = secs.rem_euclid(SECS_PER_DAY);
    let (hour, minute) = (of_day / 3_600, of_day % 3_600 / 60);
    let minute = format!("{minute:02}");
    match locale.hour_cycle() {
        HourCycle::H24 => format!("{hour:02}:{minute}"),
        HourCycle::H12 => {
            let period = if hour < 12 { "period-am" } else { "period-pm" };
            let period = locale.tr(period, &[]);
            let args = [
                ("hour", Arg::Num((hour + 11) % 12 + 1)),
                ("minute", Arg::Str(&minute)),
                ("period", Arg::Str(&period)),
            ];
            locale.tr("time-12h", &args)
        }
    }
}

/// `date-short` of a day, in the order of the language.
pub fn short_date(day: Day, locale: &Locale) -> String {
    let two = |n: i64| format!("{:02}", n.rem_euclid(100));
    let (d, m, y) = (
        two(i64::from(day.day)),
        two(i64::from(day.month)),
        two(i64::from(day.year)),
    );
    let args = [
        ("day", Arg::Str(&d)),
        ("month", Arg::Str(&m)),
        ("year", Arg::Str(&y)),
    ];
    locale.tr("date-short", &args)
}

/// The key of the short name of the weekday of a day number.
pub fn weekday_key(day_number: i64) -> &'static str {
    const KEYS: [&str; 7] = [
        "weekday-short-mon",
        "weekday-short-tue",
        "weekday-short-wed",
        "weekday-short-thu",
        "weekday-short-fri",
        "weekday-short-sat",
        "weekday-short-sun",
    ];
    // 1970-01-01 was a Thursday.
    KEYS[(day_number + 3).rem_euclid(7) as usize]
}

/// Whole days from `day` to the day of the reader at `now`.
pub fn age_in_days(day: Day, now: Timestamp, locale: &Locale) -> i64 {
    local_day(now, locale) - day_number(day)
}
