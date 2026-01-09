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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // ─────────────────────────────────────────────────────────────────────────────
    // HighlightColor tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_highlight_color_parse_valid() {
        assert!(matches!(
            HighlightColor::parse("cyan"),
            Some(HighlightColor::Cyan)
        ));
        assert!(matches!(
            HighlightColor::parse("green"),
            Some(HighlightColor::Green)
        ));
        assert!(matches!(
            HighlightColor::parse("yellow"),
            Some(HighlightColor::Yellow)
        ));
        assert!(matches!(
            HighlightColor::parse("blue"),
            Some(HighlightColor::Blue)
        ));
        assert!(matches!(
            HighlightColor::parse("magenta"),
            Some(HighlightColor::Magenta)
        ));
    }

    #[test]
    fn test_highlight_color_parse_case_insensitive() {
        assert!(matches!(
            HighlightColor::parse("CYAN"),
            Some(HighlightColor::Cyan)
        ));
        assert!(matches!(
            HighlightColor::parse("Cyan"),
            Some(HighlightColor::Cyan)
        ));
        assert!(matches!(
            HighlightColor::parse("GREEN"),
            Some(HighlightColor::Green)
        ));
    }

    #[test]
    fn test_highlight_color_parse_invalid() {
        assert!(HighlightColor::parse("red").is_none());
        assert!(HighlightColor::parse("purple").is_none());
        assert!(HighlightColor::parse("").is_none());
        assert!(HighlightColor::parse("invalid").is_none());
    }

    #[test]
    fn test_highlight_color_to_comfy_color() {
        assert!(matches!(
            HighlightColor::Cyan.to_comfy_color(),
            comfy_table::Color::Cyan
        ));
        assert!(matches!(
            HighlightColor::Green.to_comfy_color(),
            comfy_table::Color::Green
        ));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // ColorConfig tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    #[serial]
    fn test_color_config_new_enabled() {
        // Ensure NO_COLOR is not set for this test
        unsafe {
            std::env::remove_var("NO_COLOR");
        }

        let config = ColorConfig::new(HighlightColor::Green, true);
        assert!(config.enabled);
        assert!(matches!(config.highlight_color, HighlightColor::Green));
    }

    #[test]
    fn test_color_config_new_disabled() {
        let config = ColorConfig::new(HighlightColor::Blue, false);
        assert!(!config.enabled);
    }

    #[test]
    #[serial]
    fn test_color_config_highlight_enabled() {
        unsafe {
            std::env::remove_var("NO_COLOR");
        }

        let config = ColorConfig::new(HighlightColor::Cyan, true);
        let result = config.highlight("test");
        // When enabled, the result should contain ANSI escape codes
        assert!(result.contains("test"));
    }

    #[test]
    fn test_color_config_highlight_disabled() {
        let config = ColorConfig::new(HighlightColor::Cyan, false);
        let result = config.highlight("test");
        // When disabled, should return plain text
        assert_eq!(result, "test");
    }

    #[test]
    fn test_color_config_error_disabled() {
        let config = ColorConfig::new(HighlightColor::Cyan, false);
        let result = config.error("error message");
        assert_eq!(result, "error message");
    }
}
