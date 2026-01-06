use colored::*;
use std::env;

#[derive(Debug, Clone, Copy)]
pub enum HighlightColor {
    Cyan,
    Green,
    Yellow,
    Blue,
    Magenta,
}

impl HighlightColor {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cyan" => Some(Self::Cyan),
            "green" => Some(Self::Green),
            "yellow" => Some(Self::Yellow),
            "blue" => Some(Self::Blue),
            "magenta" => Some(Self::Magenta),
            _ => None,
        }
    }

    pub fn apply(&self, text: &str) -> ColoredString {
        match self {
            Self::Cyan => text.cyan(),
            Self::Green => text.green(),
            Self::Yellow => text.yellow(),
            Self::Blue => text.blue(),
            Self::Magenta => text.magenta(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorConfig {
    pub highlight_color: HighlightColor,
    pub enabled: bool,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            highlight_color: HighlightColor::Cyan,
            enabled: env::var("NO_COLOR").is_err(),
        }
    }
}

impl ColorConfig {
    pub fn new(highlight_color: HighlightColor, enabled: bool) -> Self {
        // Respect NO_COLOR env var: if it's set, disable colors regardless of the enabled flag
        let respects_no_color = env::var("NO_COLOR").is_ok();
        Self {
            highlight_color,
            enabled: enabled && !respects_no_color,
        }
    }

    pub fn highlight(&self, text: &str) -> String {
        if self.enabled {
            self.highlight_color.apply(text).bold().to_string()
        } else {
            text.to_string()
        }
    }

    pub fn error(&self, text: &str) -> String {
        if self.enabled {
            text.red().to_string()
        } else {
            text.to_string()
        }
    }
}
