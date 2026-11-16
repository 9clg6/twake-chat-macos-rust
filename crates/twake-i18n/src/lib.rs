//! Translator, locale detection and plural rules.
//! The catalogue a view model asks for its texts (A006, A016: Fluent).

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use unic_langid::LanguageIdentifier;

/// A value given to a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg<'a> {
    /// A text.
    Str(&'a str),
    /// A number; it picks the plural form.
    Num(i64),
}

/// How the clock is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HourCycle {
    /// `14:05`.
    #[default]
    H24,
    /// `2:05 PM`.
    H12,
}

const EN_FTL: &str = include_str!("../locales/en.ftl");
const FR_FTL: &str = include_str!("../locales/fr.ftl");

/// One language: its catalogue, the offset of the reader and the way the clock reads.
pub struct Locale {
    bundle: FluentBundle<FluentResource>,
    utc_offset_secs: i32,
    hour_cycle: HourCycle,
}

impl Locale {
    /// One Fluent catalogue (`ftl` text) for `lang` (BCP 47), a reader `utc_offset_secs` east of UTC.
    /// A bad language falls back to the undetermined one; a bad entry of the catalogue is skipped.
    pub fn from_ftl(lang: &str, ftl: &str, utc_offset_secs: i32) -> Self {
        let id: LanguageIdentifier = lang.parse().unwrap_or_default();
        let mut bundle = FluentBundle::new_concurrent(vec![id]);
        // No Unicode isolation marks around the arguments.
        bundle.set_use_isolating(false);
        let resource =
            FluentResource::try_new(ftl.to_owned()).unwrap_or_else(|(partial, _)| partial);
        // A key present twice keeps the first; the errors name them and a screen must not crash.
        let _ = bundle.add_resource(resource);
        Self {
            bundle,
            utc_offset_secs,
            hour_cycle: HourCycle::default(),
        }
    }

    /// The bundled catalogue of `lang` (`fr`, `fr-CA`...); any other language gets English.
    pub fn bundled(lang: &str, utc_offset_secs: i32) -> Self {
        let french = lang.split(['-', '_']).next() == Some("fr");
        let (tag, ftl) = if french {
            ("fr", FR_FTL)
        } else {
            ("en", EN_FTL)
        };
        Self::from_ftl(tag, ftl, utc_offset_secs)
    }

    /// The same locale with another clock.
    pub fn with_hour_cycle(mut self, cycle: HourCycle) -> Self {
        self.hour_cycle = cycle;
        self
    }

    /// Seconds east of UTC of the reader.
    pub fn utc_offset_secs(&self) -> i32 {
        self.utc_offset_secs
    }

    /// The clock.
    pub fn hour_cycle(&self) -> HourCycle {
        self.hour_cycle
    }

    /// The message `key` with its arguments; an unknown key gives the key itself.
    pub fn tr(&self, key: &str, args: &[(&str, Arg)]) -> String {
        let Some(pattern) = self.bundle.get_message(key).and_then(|m| m.value()) else {
            return key.to_owned();
        };
        let mut fluent_args = FluentArgs::new();
        for (name, arg) in args {
            match arg {
                Arg::Str(text) => fluent_args.set(*name, *text),
                Arg::Num(number) => fluent_args.set(*name, *number),
            }
        }
        let mut errors = Vec::new();
        self.bundle
            .format_pattern(pattern, Some(&fluent_args), &mut errors)
            .into_owned()
    }
}
