use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub width_pct: u16,
    pub height_pct: u16,
    pub key_hint_workspace: String,
    pub key_hint_session: String,
    pub key_hint_minimize: String,
    pub backdrop: Rgb,
    pub forward_inner_mouse: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            width_pct: 94,
            height_pct: 92,
            key_hint_workspace: "prefix+f".into(),
            key_hint_session: "prefix+shift+f".into(),
            key_hint_minimize: "prefix+cmd+z".into(),
            backdrop: Rgb(0x0d, 0x2b, 0x1d),
            forward_inner_mouse: true,
        }
    }
}

impl AppConfig {
    pub fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
        let mut cfg: Self = toml::from_str(input)?;
        cfg.normalize();
        Ok(cfg)
    }

    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn with_size(&self, width_pct: u16, height_pct: u16) -> Self {
        let mut next = self.clone();
        next.width_pct = clamp_pct(width_pct);
        next.height_pct = clamp_pct(height_pct);
        next
    }

    fn normalize(&mut self) {
        self.width_pct = clamp_pct(self.width_pct);
        self.height_pct = clamp_pct(self.height_pct);
        if self.key_hint_workspace.is_empty() {
            self.key_hint_workspace = Self::default().key_hint_workspace;
        }
        if self.key_hint_session.is_empty() {
            self.key_hint_session = Self::default().key_hint_session;
        }
        if self.key_hint_minimize.is_empty() {
            self.key_hint_minimize = Self::default().key_hint_minimize;
        }
    }
}

fn clamp_pct(value: u16) -> u16 {
    value.clamp(20, 100)
}

impl Serialize for Rgb {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2))
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        parse_rgb(&raw).ok_or_else(|| serde::de::Error::custom("expected #rrggbb color"))
    }
}

fn parse_rgb(value: &str) -> Option<Rgb> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(Rgb(
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ))
}
