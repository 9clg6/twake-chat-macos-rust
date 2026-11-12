//! The user's display and notification choices, and what the screens read from them (FR-128,
//! FR-129). Pure: the OS says what it prefers through `SystemEnv`, the document is kept by a port.

mod json;

/// Version of the JSON document written by `Preferences::to_json`.
pub const SCHEMA_VERSION: u32 = 2;

/// The languages of the app, in the order of the picker.
pub const SUPPORTED_LANGUAGES: [&str; 7] = ["en", "fr", "de", "es", "it", "ru", "vi"];

/// A closed set of choices, each with its word in the JSON document.
trait Word: Sized + Copy {
    fn word(self) -> &'static str;
    fn parse(word: &str) -> Option<Self>;
}

macro_rules! words {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $word:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $(
                #[doc = concat!("Written `", $word, "` in the document.")]
                $variant,
            )+
        }

        impl Word for $name {
            fn word(self) -> &'static str {
                match self { $(Self::$variant => $word),+ }
            }

            fn parse(word: &str) -> Option<Self> {
                match word { $($word => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}

words!(
    /// Which colours the user asks for.
    ThemeChoice { System = "system", Light = "light", Dark = "dark" }
);
words!(
    /// Spacing of the lists and bubbles.
    Density { Comfortable = "comfortable", Compact = "compact" }
);
words!(
    /// 12 or 24 hour clock.
    HourCycle { System = "system", H12 = "12h", H24 = "24h" }
);
words!(
    /// Size of the text.
    TextSize { Small = "small", Normal = "normal", Large = "large" }
);
words!(
    /// How the room list is sorted.
    RoomOrder { Recent = "recent", Sections = "sections" }
);
words!(
    /// Background of the conversation.
    Wallpaper { Default = "default", None = "none" }
);
words!(
    /// Side of the user's own bubbles.
    BubbleAlign { OwnRight = "own-right", Left = "left" }
);
words!(
    /// The account default for the rooms that do not choose (D22).
    GlobalNotificationLevel { All = "all", Mentions = "mentions" }
);

/// The colours a screen is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    /// Light colours.
    Light,
    /// Dark colours.
    Dark,
}

/// The language of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// The language of the OS.
    System,
    /// An entry of `SUPPORTED_LANGUAGES`.
    Code(&'static str),
}

impl TextSize {
    /// The factor applied to every font size (D272).
    pub fn scale(self) -> f32 {
        match self {
            Self::Small => 0.9,
            Self::Normal => 1.0,
            Self::Large => 1.15,
        }
    }
}

/// Every choice of the Settings screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preferences {
    /// Light, dark or as the system.
    pub theme: ThemeChoice,
    /// Comfortable or compact.
    pub density: Density,
    /// Language of the interface.
    pub language: Language,
    /// 12 or 24 hour clock.
    pub time_format: HourCycle,
    /// Size of the text.
    pub text_size: TextSize,
    /// Order of the room list.
    pub room_order: RoomOrder,
    /// Background of the conversation.
    pub wallpaper: Wallpaper,
    /// Side of the user's own bubbles.
    pub bubble_align: BubbleAlign,
    /// Colours drained.
    pub grayscale: bool,
    /// `None`: as the system (D437).
    pub high_contrast: Option<bool>,
    /// Silences every notification.
    pub do_not_disturb: bool,
    /// `None`: as reduced motion (D259).
    pub autoplay: Option<bool>,
    /// Read receipts are not sent (D55).
    pub private_receipts: bool,
    /// Default level of the direct rooms.
    pub direct_messages: GlobalNotificationLevel,
    /// Default level of the groups.
    pub groups: GlobalNotificationLevel,
    /// A sound for a new message (D240).
    pub message_sound: bool,
    /// The text of the message shows in the notification.
    pub notification_text: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::System,
            density: Density::Comfortable,
            language: Language::System,
            time_format: HourCycle::System,
            text_size: TextSize::Normal,
            room_order: RoomOrder::Recent,
            wallpaper: Wallpaper::Default,
            bubble_align: BubbleAlign::OwnRight,
            grayscale: false,
            high_contrast: None,
            do_not_disturb: false,
            autoplay: None,
            private_receipts: false,
            direct_messages: GlobalNotificationLevel::All,
            groups: GlobalNotificationLevel::All,
            message_sound: true,
            notification_text: true,
        }
    }
}

/// What the OS says; the "as the system" choices read it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemEnv {
    /// The system is in dark mode.
    pub dark: bool,
    /// The system asks for more contrast.
    pub high_contrast: bool,
    /// The system asks for less motion.
    pub reduced_motion: bool,
    /// The system clock is a 24 hour one.
    pub hour_24: bool,
    /// Language of the OS as it gives it: `fr-FR`, `de_AT`, or empty.
    pub language: String,
    /// Common settings of ToM (D38), `None` = none known.
    pub common: Option<CommonSettings>,
}

/// The `user_info` of ToM, read only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonSettings {
    /// `None`: the user chose nothing there.
    pub theme: Option<CommonTheme>,
    /// Language as ToM keeps it (`fr-FR`), `None` = none.
    pub language: Option<String>,
    /// Page where they change.
    pub url: Option<String>,
}

/// The theme of the common settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonTheme {
    /// Light.
    Light,
    /// Dark.
    Dark,
    /// The theme of the system.
    Auto,
}

/// What a poll of ToM gave.
pub enum CommonFetch {
    /// ToM answered.
    Reached(Option<CommonSettings>),
    /// ToM did not answer.
    Unreachable,
}

impl SystemEnv {
    /// ToM down is not "no theme" (D38).
    pub fn with_common_fetch(self, fetch: CommonFetch) -> SystemEnv {
        match fetch {
            CommonFetch::Reached(common) => SystemEnv { common, ..self },
            CommonFetch::Unreachable => self,
        }
    }
}

/// The page of the common settings of `user_id`, http(s) only.
pub fn common_settings_url(template: &str, user_id: &str) -> Option<String> {
    let username = user_id.strip_prefix('@')?.split_once(':')?.0;
    if username.is_empty() || !template.contains("{username}") {
        return None;
    }
    let url = template.replace("{username}", &encode(username));
    let (scheme, rest) = url.split_once("://")?;
    let scheme = scheme.to_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (host, tail) = rest.split_at(end);
    if host.is_empty() || host.contains(' ') {
        return None;
    }
    let tail = if tail.starts_with('/') {
        tail.to_string()
    } else {
        format!("/{tail}")
    };
    Some(format!("{scheme}://{}{tail}", host.to_lowercase()))
}

fn encode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Per kind of room (D22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationDefaults {
    /// Direct rooms.
    pub direct_messages: GlobalNotificationLevel,
    /// Groups.
    pub groups: GlobalNotificationLevel,
}

impl Preferences {
    /// The account defaults per kind of room.
    pub fn notification_defaults(&self) -> NotificationDefaults {
        NotificationDefaults {
            direct_messages: self.direct_messages,
            groups: self.groups,
        }
    }
}

/// Everything a screen reads, derived in one place.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewSettings {
    /// Colours to draw with.
    pub scheme: ColorScheme,
    /// Spacing.
    pub density: Density,
    /// Factor of the font sizes.
    pub text_scale: f32,
    /// An entry of `SUPPORTED_LANGUAGES`.
    pub language: &'static str,
    /// 24 hour clock.
    pub hour_24: bool,
    /// More contrast.
    pub high_contrast: bool,
    /// Colours drained.
    pub grayscale: bool,
    /// Animated media play by themselves.
    pub autoplay: bool,
    /// Forced to `None` while high contrast is on (D437).
    pub wallpaper: Wallpaper,
    /// Order of the room list.
    pub room_order: RoomOrder,
    /// Side of the user's own bubbles.
    pub bubble_align: BubbleAlign,
    /// Notifications are silenced.
    pub do_not_disturb: bool,
    /// A common theme is in place: Appearance is disabled.
    pub appearance_locked: bool,
    /// A supported common language is in place: the picker is disabled.
    pub language_locked: bool,
    /// Link to the common settings, while locked.
    pub common_settings_url: Option<String>,
}

impl Preferences {
    /// The colours: `System` follows the OS.
    pub fn resolve_theme(&self, system_is_dark: bool) -> ColorScheme {
        let dark = match self.theme {
            ThemeChoice::System => system_is_dark,
            ThemeChoice::Light => false,
            ThemeChoice::Dark => true,
        };
        if dark {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        }
    }

    /// A choice of the user wins over the system.
    pub fn resolve_high_contrast(&self, system_wants: bool) -> bool {
        self.high_contrast.unwrap_or(system_wants)
    }

    /// A choice of the user wins; otherwise on, unless the user reduces motion.
    pub fn resolve_autoplay(&self, reduced_motion: bool) -> bool {
        self.autoplay.unwrap_or(!reduced_motion)
    }

    /// The language to show: the chosen one, or the OS one when supported, else English.
    pub fn resolve_language(&self, system: &str) -> &'static str {
        match self.language {
            Language::Code(code) => code,
            Language::System => system_language(system),
        }
    }

    /// 12 hour clock, 24 hour clock, or the system one.
    pub fn resolve_hour_24(&self, system_is_24h: bool) -> bool {
        match self.time_format {
            HourCycle::System => system_is_24h,
            HourCycle::H12 => false,
            HourCycle::H24 => true,
        }
    }

    /// The one derivation every screen reads (D349).
    pub fn view(&self, env: &SystemEnv) -> ViewSettings {
        let high_contrast = self.resolve_high_contrast(env.high_contrast);
        let common = env.common.as_ref();
        let common_theme = common.and_then(|c| c.theme);
        let shared_language = common.and_then(shared_language);
        let locked = common_theme.is_some() || shared_language.is_some();
        ViewSettings {
            scheme: common_theme
                .map_or_else(|| self.resolve_theme(env.dark), |t| t.scheme(env.dark)),
            appearance_locked: common_theme.is_some(),
            language_locked: shared_language.is_some(),
            common_settings_url: common.filter(|_| locked).and_then(|c| c.url.clone()),
            density: self.density,
            text_scale: self.text_size.scale(),
            language: shared_language.unwrap_or_else(|| self.resolve_language(&env.language)),
            hour_24: self.resolve_hour_24(env.hour_24),
            high_contrast,
            grayscale: self.grayscale,
            autoplay: self.resolve_autoplay(env.reduced_motion),
            wallpaper: if high_contrast {
                Wallpaper::None
            } else {
                self.wallpaper
            },
            room_order: self.room_order,
            bubble_align: self.bubble_align,
            do_not_disturb: self.do_not_disturb,
        }
    }
}

impl CommonTheme {
    /// `Auto` follows the system.
    fn scheme(self, system_is_dark: bool) -> ColorScheme {
        match self {
            Self::Light => ColorScheme::Light,
            Self::Dark => ColorScheme::Dark,
            Self::Auto if system_is_dark => ColorScheme::Dark,
            Self::Auto => ColorScheme::Light,
        }
    }
}

/// The supported language of the common settings, if there is one (D67).
fn shared_language(common: &CommonSettings) -> Option<&'static str> {
    common.language.as_deref().and_then(base_language)
}

/// `fr-FR` and `DE_at` give `fr` and `de`; anything unsupported gives `en`.
fn system_language(system: &str) -> &'static str {
    let base = system.split(['-', '_']).next().unwrap_or_default();
    supported(&base.to_lowercase()).unwrap_or(SUPPORTED_LANGUAGES[0])
}

/// The entry of `SUPPORTED_LANGUAGES` that is `code`.
fn supported(code: &str) -> Option<&'static str> {
    SUPPORTED_LANGUAGES.into_iter().find(|known| *known == code)
}

/// The first two letters, lower-cased, as the React app reads it.
fn base_language(language: &str) -> Option<&'static str> {
    let base: String = language.chars().take(2).collect();
    supported(&base.to_lowercase())
}
