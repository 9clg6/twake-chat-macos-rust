//! The JSON document of the preferences: written in v2, read tolerantly, v1 migrated.

use serde_json::{Map, Value, json};

use super::{Language, Preferences, SCHEMA_VERSION, Word, supported};

type Fields = Map<String, Value>;

impl Preferences {
    /// The document, always at the current version; the same value gives the same text.
    pub fn to_json(&self) -> String {
        let language = match self.language {
            Language::System => "system",
            Language::Code(code) => code,
        };
        json!({
            "version": SCHEMA_VERSION,
            "theme": self.theme.word(),
            "density": self.density.word(),
            "language": language,
            "time_format": self.time_format.word(),
            "text_size": self.text_size.word(),
            "room_order": self.room_order.word(),
            "wallpaper": self.wallpaper.word(),
            "bubble_align": self.bubble_align.word(),
            "grayscale": self.grayscale,
            "high_contrast": self.high_contrast,
            "do_not_disturb": self.do_not_disturb,
            "autoplay": self.autoplay,
            "private_receipts": self.private_receipts,
            "notifications": self.notifications.word(),
            "message_sound": self.message_sound,
            "notification_text": self.notification_text,
        })
        .to_string()
    }

    /// Reads a document of any version. Never fails: what cannot be read is the default, field
    /// by field (a text that is no object is all defaults).
    pub fn from_json(text: &str) -> Self {
        match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(mut fields)) => {
                upgrade(&mut fields);
                read_fields(&fields)
            }
            _ => Self::default(),
        }
    }
}

/// Puts a document with no version, or version 1, in the shape of v2.
fn upgrade(fields: &mut Fields) {
    let version = fields.get("version").and_then(Value::as_u64);
    if version.is_some_and(|v| v >= 2) {
        return;
    }
    fields.remove("time_format");
    if let Some(clock) = fields.remove("clock") {
        let clock = if clock == "auto" {
            json!("system")
        } else {
            clock
        };
        fields.insert("time_format".into(), clock);
    }
    if fields
        .get("room_order")
        .is_some_and(|order| order == "category")
    {
        fields.insert("room_order".into(), json!("sections"));
    }
}

fn read_fields(fields: &Fields) -> Preferences {
    let defaults = Preferences::default();
    Preferences {
        theme: choice(fields, "theme", defaults.theme),
        density: choice(fields, "density", defaults.density),
        language: language(fields),
        time_format: choice(fields, "time_format", defaults.time_format),
        text_size: choice(fields, "text_size", defaults.text_size),
        room_order: choice(fields, "room_order", defaults.room_order),
        wallpaper: choice(fields, "wallpaper", defaults.wallpaper),
        bubble_align: choice(fields, "bubble_align", defaults.bubble_align),
        grayscale: flag(fields, "grayscale", defaults.grayscale),
        high_contrast: fields.get("high_contrast").and_then(Value::as_bool),
        do_not_disturb: flag(fields, "do_not_disturb", defaults.do_not_disturb),
        autoplay: fields.get("autoplay").and_then(Value::as_bool),
        private_receipts: flag(fields, "private_receipts", defaults.private_receipts),
        notifications: choice(fields, "notifications", defaults.notifications),
        message_sound: flag(fields, "message_sound", defaults.message_sound),
        notification_text: flag(fields, "notification_text", defaults.notification_text),
    }
}

fn choice<T: Word>(fields: &Fields, key: &str, default: T) -> T {
    fields
        .get(key)
        .and_then(Value::as_str)
        .and_then(T::parse)
        .unwrap_or(default)
}

fn flag(fields: &Fields, key: &str, default: bool) -> bool {
    fields.get(key).and_then(Value::as_bool).unwrap_or(default)
}

/// A language that is not supported is the system one (D67).
fn language(fields: &Fields) -> Language {
    fields
        .get("language")
        .and_then(Value::as_str)
        .and_then(supported)
        .map_or(Language::System, Language::Code)
}
