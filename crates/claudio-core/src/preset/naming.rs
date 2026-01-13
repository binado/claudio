use std::path::PathBuf;

pub(crate) fn is_legacy_filename_safe(name: &str) -> bool {
    // Legacy behavior used `<name>.json` directly.
    // Only allow that mapping when it cannot traverse directories.
    if name.is_empty() {
        return false;
    }
    if name.contains('\0') {
        return false;
    }
    if name.contains('/') || name.contains('\\') {
        return false;
    }
    // Avoid special path components.
    if name == "." || name == ".." {
        return false;
    }
    true
}

pub(crate) fn preset_filename_stem_for_name(name: &str) -> String {
    // Percent-encode UTF-8 bytes into a filesystem-safe filename stem.
    // We leave RFC3986 "unreserved" bytes as-is: ALPHA / DIGIT / "-" / "." / "_" / "~".
    // Everything else is escaped as `%XX` (uppercase hex).
    //
    // This is deterministic and avoids path traversal even when names contain `/`.
    let bytes = name.as_bytes();
    let mut out = String::with_capacity(bytes.len());

    for &b in bytes {
        let unreserved = matches!(
            b,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~'
        );

        if unreserved {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(
                char::from_digit((b >> 4) as u32, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
            out.push(
                char::from_digit((b & 0x0F) as u32, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
        }
    }

    // Avoid empty stems and special path components.
    if out.is_empty() {
        return "%00".to_string();
    }
    if out == "." {
        return "%2E".to_string();
    }
    if out == ".." {
        return "%2E%2E".to_string();
    }

    out
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
