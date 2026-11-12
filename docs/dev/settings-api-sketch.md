# Settings core API sketch: pure preferences, notification levels, port, state

Written by the test author from the need, before the production code exists. This is the exact
public surface the tests use. The implementer may add more, but every item here must exist with
these names, types and semantics. A disagreement is argued against the need (the cited source),
not against the code.

Slice: the pure part of FR-128, FR-129, FR-133 (`docs/prd-twake-chat-macos-rust-2026-11-12.md`,
EPIC-15), behaviour per `docs/spec/features/settings.md` (D55, D67, D85, D240, D259, D261, D272,
D328, D349, D437), `theming.md` (D38), `notifications.md` (D22, D615). A009 (boxed `Send`
futures), preferences kept by `twake-store` (ARCHITECTURE.md section 2). Reference for the exact
rules, read-only: `twake-space-chat` `src/ui/displayPreferences.ts` (defaults and tolerant read),
`src/domain/notifications.ts` (levels), `src/app/languages.ts`.

No UI, no disk, no clock, no manifest change (`serde_json` is already a dependency of
`twake-domain`; `futures` of `twake-app`).

Tests: `crates/twake-domain/tests/{preferences,preferences_json,notification_level}.rs`,
`crates/twake-app/tests/settings_state.rs`, `crates/twake-testing/tests/preferences_store.rs` and
the contract suite `crates/twake-testing/src/preferences_contract.rs` (written, not to be loosened;
the implementer adds `mod preferences_contract;` to `lib.rs`,
`pub use crate::preferences_contract::preferences;` to `contract.rs`, `FakePreferences`, and
re-exports it).

---

## 1. `twake_domain::preferences` (all fields `pub`, no `#[non_exhaustive]`)

```rust
pub const SCHEMA_VERSION: u32 = 2;
/// From the React app (`SUPPORTED_LANGUAGES`), in this order.
pub const SUPPORTED_LANGUAGES: [&str; 7] = ["en", "fr", "de", "es", "it", "ru", "vi"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ThemeChoice { System, Light, Dark }   // default System
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ColorScheme { Light, Dark }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Density { Comfortable, Compact }      // default Comfortable
/// `System`: the language of the OS. `Code` only ever holds an entry of SUPPORTED_LANGUAGES.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Language { System, Code(&'static str) }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum HourCycle { System, H12, H24 }        // default System
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum TextSize { Small, Normal, Large }     // default Normal
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum RoomOrder { Recent, Sections }        // default Recent
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Wallpaper { Default, None }           // default Default
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum BubbleAlign { OwnRight, Left }        // default OwnRight
/// The account default for rooms that do not choose (D22; `directMessages`/`groups` of React).
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum GlobalNotificationLevel { All, Mentions } // default All

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preferences {
    pub theme: ThemeChoice,                // System
    pub density: Density,                  // Comfortable
    pub language: Language,                // System
    pub time_format: HourCycle,            // System
    pub text_size: TextSize,               // Normal
    pub room_order: RoomOrder,             // Recent
    pub wallpaper: Wallpaper,              // Default
    pub bubble_align: BubbleAlign,         // OwnRight
    pub grayscale: bool,                   // false
    pub high_contrast: Option<bool>,       // None = as the system (D437)
    pub do_not_disturb: bool,              // false
    pub autoplay: Option<bool>,            // None = as reduced motion (D259)
    pub private_receipts: bool,            // false (D55)
    pub notifications: GlobalNotificationLevel, // All
    pub message_sound: bool,               // true (D240)
    pub notification_text: bool,           // true (settings.md, display.e2e "notification-text")
}
impl Default for Preferences { /* the values above */ }

/// What the OS says; the user's "as the system" choices read it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemEnv {
    pub dark: bool,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub hour_24: bool,
    pub language: String,    // BCP 47 as the OS gives it: "fr-FR", "de_AT", ""
}

/// Everything a screen reads, derived in one place from `Preferences` + `SystemEnv`.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewSettings {
    pub scheme: ColorScheme,
    pub density: Density,
    pub text_scale: f32,            // Small 0.9, Normal 1.0, Large 1.15 (D272)
    pub language: &'static str,     // always an entry of SUPPORTED_LANGUAGES
    pub hour_24: bool,
    pub high_contrast: bool,
    pub grayscale: bool,
    pub autoplay: bool,
    pub wallpaper: Wallpaper,       // forced to None while high_contrast is on (D437)
    pub room_order: RoomOrder,
    pub bubble_align: BubbleAlign,
    pub do_not_disturb: bool,
}

impl Preferences {
    /// Theme: System follows `system_is_dark`.
    pub fn resolve_theme(&self, system_is_dark: bool) -> ColorScheme;
    /// `Some(x)` wins over the system.
    pub fn resolve_high_contrast(&self, system_wants: bool) -> bool;
    /// `Some(x)` wins; None = on unless the user reduces motion.
    pub fn resolve_autoplay(&self, reduced_motion: bool) -> bool;
    /// `Code(c)` -> c. System -> the part of `system` before `-` or `_`, lower-cased, if supported
    /// ("fr-FR" -> "fr", "DE_at" -> "de"), else "en" ("ja", "pt-BR", "").
    pub fn resolve_language(&self, system: &str) -> &'static str;
    /// H12 -> false, H24 -> true, System -> `system_is_24h`.
    pub fn resolve_hour_24(&self, system_is_24h: bool) -> bool;
    pub fn view(&self, env: &SystemEnv) -> ViewSettings;
}
impl TextSize { pub fn scale(self) -> f32; }

// JSON (document kept by the store). Never fails.
impl Preferences {
    pub fn to_json(&self) -> String;            // same value -> same text; always "version": 2
    pub fn from_json(text: &str) -> Preferences;
}
```

### JSON document (version 2)

Object, keys exactly: `version` (2), `theme` ("system"|"light"|"dark"), `density`
("comfortable"|"compact"), `language` ("system" or a supported code), `time_format`
("system"|"12h"|"24h"), `text_size` ("small"|"normal"|"large"), `room_order`
("recent"|"sections"), `wallpaper` ("default"|"none"), `bubble_align` ("own-right"|"left"),
`grayscale`, `do_not_disturb`, `private_receipts`, `message_sound`, `notification_text` (booleans),
`high_contrast`, `autoplay` (true, false or null), `notifications` ("all"|"mentions").
Keys are plain: the `app.twake.chat.*` names of the spec are Matrix account-data events (D55, D259),
not this local file.

`from_json` is tolerant: not JSON, or not an object -> all defaults; unknown keys ignored; a
missing key or a value of the wrong type or of an unknown name -> the default of **that field
alone**; a language that is not supported -> `System` (D67). Reads for any version, the future
ones included, with the v2 rules.

### Migration v1 -> v2

A document with no `version`, or `version` 1, is read with the v1 rules, then is a v2 value
(`to_json` writes v2):
- `clock` ("auto"|"12h"|"24h") is `time_format` ("system"|"12h"|"24h"); `time_format` is not read;
- `room_order` "category" is "sections" (the former name in React `readDisplayPreferences`);
- every other key reads as in v2. In v2 `clock` is an unknown key and "category" an unknown name.

## 2. `twake_domain::notifications`

```rust
/// What a room does: its own choice, or the account default.
pub enum RoomNotificationLevel { Default, All, Mentions, None }
/// The level that applies.
pub enum NotificationLevel { All, Mentions, None }

/// Default -> the account default; All/Mentions/None -> themselves.
pub fn effective_level(room: RoomNotificationLevel, global: GlobalNotificationLevel) -> NotificationLevel;

impl NotificationLevel {
    /// All: yes. Mentions: only a mention. None: never, a mention neither (D22).
    pub fn notifies(self, is_mention: bool) -> bool;
    /// Mentions and None: the badge is grey (D615).
    pub fn is_muted(self) -> bool;
}
```
(Own messages never notify: the caller's rule, as React `shouldNotify`; not here.)

## 3. `twake_app::ports`

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PreferencesError { #[error("the preferences are unavailable")] Unavailable }

/// The preferences document (A009).
pub trait PreferencesPort: Send + Sync {
    /// Nothing stored yet: `Preferences::default()`.
    fn load(&self) -> BoxFuture<'_, Result<Preferences, PreferencesError>>;
    /// The whole document replaces the old one, atomically: a reader sees the old or the new
    /// document, never a mix. Last write wins.
    fn save(&self, preferences: Preferences) -> BoxFuture<'_, Result<(), PreferencesError>>;
    /// First item: the current document; then one item per real change (a save of a value equal
    /// to the stored one notifies nothing). Each subscription is independent.
    fn subscribe(&self) -> BoxFuture<'_, Result<BoxStream<'static, Preferences>, PreferencesError>>;
}
```

`twake_testing::FakePreferences`: `new()`, `Default`, `unavailable()` (every call fails with
`Unavailable`, nothing kept), `with(Preferences)` (seeded). Contract:
`twake_testing::contract::preferences(make: impl Fn() -> Arc<dyn PreferencesPort>) -> Verdict`
(`make` builds an empty store).

## 4. `twake_app::settings::SettingsState`

Plain value. `current` is what the screens show (every change applies at once, nothing waits for
the port); `saved` is the last document known to be stored.

```rust
impl SettingsState {
    pub fn new(saved: Preferences) -> Self;          // current == saved, not dirty
    pub fn preferences(&self) -> &Preferences;       // current
    pub fn saved(&self) -> &Preferences;
    pub fn is_dirty(&self) -> bool;                  // current != saved
    /// One setter per field, same name: set_theme, set_density, set_language, set_time_format,
    /// set_text_size, set_room_order, set_wallpaper, set_bubble_align, set_grayscale,
    /// set_high_contrast(Option<bool>), set_do_not_disturb, set_autoplay(Option<bool>),
    /// set_private_receipts, set_notifications, set_message_sound, set_notification_text.
    pub fn set_density(&mut self, density: Density);
    pub fn cancel(&mut self);                        // current = saved
    /// The document that was written (clone of `preferences()` taken before the save). Edits
    /// made meanwhile stay dirty.
    pub fn mark_saved(&mut self, written: Preferences);
    /// A document arrived from the port stream: it becomes `saved`; `current` follows it only
    /// when the user had no pending change, else `current` is left alone (ponytail: no
    /// field-by-field merge).
    pub fn apply_external(&mut self, stored: Preferences);
    /// The one derivation every screen reads (D349: no copy, no per-screen choice).
    pub fn view(&self, env: &SystemEnv) -> ViewSettings;   // == self.preferences().view(env)
}
```

## 5. Open questions

See the report of the test author (gaps between PRD and React, `directMessages`/`groups` split,
push-rule switches D85, bubble style, "reduced motion" as a setting).
