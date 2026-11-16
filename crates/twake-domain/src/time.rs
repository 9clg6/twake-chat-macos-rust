//! Instants and calendar days.

/// An instant, in milliseconds since the Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(u64);

impl Timestamp {
    /// The instant `ms` milliseconds after the epoch.
    pub fn from_millis(ms: u64) -> Self {
        Self(ms)
    }

    /// Milliseconds since the epoch.
    pub fn as_millis(self) -> u64 {
        self.0
    }
}

/// A calendar day; the derived order is the calendar order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day {
    /// The year.
    pub year: i32,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day of the month, 1 to 31.
    pub day: u8,
}

const SECS_PER_DAY: i64 = 86_400;

impl Day {
    /// The calendar day of `ts` for a reader `utc_offset_secs` east of UTC (proleptic Gregorian).
    pub fn of(ts: Timestamp, utc_offset_secs: i32) -> Self {
        // ponytail: u64 milliseconds beyond i64 seconds (year 292 billion) wrap; Matrix times are far below.
        let secs = (ts.as_millis() / 1_000) as i64 + i64::from(utc_offset_secs);
        Self::from_days_since_epoch(secs.div_euclid(SECS_PER_DAY))
    }

    /// Howard Hinnant's `civil_from_days`.
    fn from_days_since_epoch(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let day_of_era = z.rem_euclid(146_097);
        let year_of_era =
            (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let shifted_month = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
        let month = if shifted_month < 10 {
            shifted_month + 3
        } else {
            shifted_month - 9
        };
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Self {
            year: year as i32,
            month: month as u8,
            day: day as u8,
        }
    }
}
