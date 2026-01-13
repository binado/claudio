use std::path::PathBuf;

/// Check if a preset name is safe to use as a direct filename (legacy behavior).
///
/// Returns false for names that could cause path traversal or filesystem issues.
pub(crate) fn is_legacy_filename_safe(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('\0')
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != ".."
}

/// Percent-encode a preset name into a filesystem-safe filename stem.
///
/// Uses RFC3986 "unreserved" characters (ALPHA / DIGIT / "-" / "." / "_" / "~") as-is.
/// Everything else is escaped as `%XX` (uppercase hex).
pub(crate) fn preset_filename_stem_for_name(name: &str) -> String {
    fn is_unreserved(b: u8) -> bool {
        matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~')
    }

    fn encode_byte(b: u8) -> [char; 3] {
        let high = char::from_digit((b >> 4) as u32, 16)
            .unwrap()
            .to_ascii_uppercase();
        let low = char::from_digit((b & 0x0F) as u32, 16)
            .unwrap()
            .to_ascii_uppercase();
        ['%', high, low]
    }

    let mut out = String::with_capacity(name.len());

    for &b in name.as_bytes() {
        if is_unreserved(b) {
            out.push(b as char);
        } else {
            out.extend(encode_byte(b));
        }
    }

    // Handle special cases that could cause filesystem issues
    match out.as_str() {
        "" => "%00".to_string(),
        "." => "%2E".to_string(),
        ".." => "%2E%2E".to_string(),
        _ => out,
    }
}

pub(crate) fn preset_path_for_name_encoded(dir: &std::path::Path, name: &str) -> PathBuf {
    let stem = preset_filename_stem_for_name(name);
    dir.join(format!("{}.json", stem))
}

pub(crate) fn preset_path_for_name_legacy(dir: &std::path::Path, name: &str) -> PathBuf {
    dir.join(format!("{}.json", name))
}

pub fn candidate_preset_paths_for_name(dir: &std::path::Path, name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();

    let encoded = preset_path_for_name_encoded(dir, name);
    out.push(encoded.clone());

    if is_legacy_filename_safe(name) {
        let legacy = preset_path_for_name_legacy(dir, name);
        if legacy != encoded {
            out.push(legacy);
        }
    }

    out
}

/// Get the preset path for a given name under a directory.
///
/// If the name is "clean" (unreserved characters only and not `.`/`..`), this will
/// naturally produce `<dir>/<name>.json`.
///
/// For other names, it produces a percent-encoded filename.
pub fn preset_path_for_name(dir: &std::path::Path, name: &str) -> PathBuf {
    preset_path_for_name_encoded(dir, name)
}
