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

    /// Convert to comfy_table::Color for table styling
    pub fn to_comfy_color(self) -> comfy_table::Color {
        match self {
            Self::Cyan => comfy_table::Color::Cyan,
            Self::Green => comfy_table::Color::Green,
            Self::Yellow => comfy_table::Color::Yellow,
            Self::Blue => comfy_table::Color::Blue,
            Self::Magenta => comfy_table::Color::Magenta,
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
        let no_color_set = env::var("NO_COLOR").is_ok();
        Self {
            highlight_color,
            enabled: enabled && !no_color_set,
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
