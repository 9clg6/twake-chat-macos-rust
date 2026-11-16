//! The state of the Settings screen: what is shown now, and what is known to be stored.

use twake_domain::preferences::{
    BubbleAlign, Density, GlobalNotificationLevel, HourCycle, Language, Preferences, RoomOrder,
    SystemEnv, TextSize, ThemeChoice, ViewSettings, Wallpaper,
};

/// `current` is what the screens show (a change applies at once, nothing waits for the port);
/// `saved` is the last document known to be stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsState {
    current: Preferences,
    saved: Preferences,
}

macro_rules! setters {
    ($($setter:ident($field:ident: $type:ty)),+ $(,)?) => {
        $(
            #[doc = concat!("Changes `", stringify!($field), "` at once.")]
            pub fn $setter(&mut self, $field: $type) {
                self.current.$field = $field;
            }
        )+
    };
}

impl SettingsState {
    /// Nothing pending: `current` is `saved`.
    pub fn new(saved: Preferences) -> Self {
        Self {
            current: saved.clone(),
            saved,
        }
    }

    /// What the screens show.
    pub fn preferences(&self) -> &Preferences {
        &self.current
    }

    /// The last document known to be stored.
    pub fn saved(&self) -> &Preferences {
        &self.saved
    }

    /// There is a change not stored yet.
    pub fn is_dirty(&self) -> bool {
        self.current != self.saved
    }

    setters!(
        set_theme(theme: ThemeChoice),
        set_density(density: Density),
        set_language(language: Language),
        set_time_format(time_format: HourCycle),
        set_text_size(text_size: TextSize),
        set_room_order(room_order: RoomOrder),
        set_wallpaper(wallpaper: Wallpaper),
        set_bubble_align(bubble_align: BubbleAlign),
        set_grayscale(grayscale: bool),
        set_high_contrast(high_contrast: Option<bool>),
        set_do_not_disturb(do_not_disturb: bool),
        set_autoplay(autoplay: Option<bool>),
        set_private_receipts(private_receipts: bool),
        set_direct_messages(direct_messages: GlobalNotificationLevel),
        set_groups(groups: GlobalNotificationLevel),
        set_message_sound(message_sound: bool),
        set_notification_text(notification_text: bool),
    );

    /// Drops the pending changes.
    pub fn cancel(&mut self) {
        self.current = self.saved.clone();
    }

    /// `written` is now stored; edits made while it was written stay pending.
    pub fn mark_saved(&mut self, written: Preferences) {
        self.saved = written;
    }

    /// A document came from the port: it is the saved one, and `current` follows it unless the
    /// user has a pending change (no field-by-field merge).
    pub fn apply_external(&mut self, stored: Preferences) {
        if !self.is_dirty() {
            self.current = stored.clone();
        }
        self.saved = stored;
    }

    /// The one derivation every screen reads (D349).
    pub fn view(&self, env: &SystemEnv) -> ViewSettings {
        self.current.view(env)
    }
}
