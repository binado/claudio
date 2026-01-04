use crate::preset::loader;
use anyhow::Result;

pub fn which(preset_name: &str) -> Result<()> {
    let matches = loader::find_all_presets(preset_name);

    if matches.is_empty() {
        let dirs = loader::get_preset_dirs();
        let mut error_msg = format!("Preset '{}' not found in:\n", preset_name);
        for dir in dirs {
            error_msg.push_str(&format!("  - {}\n", dir.display()));
        }
        anyhow::bail!(error_msg.trim().to_string());
    }

    println!("{}", matches[0].display());

    if matches.len() > 1 {
        println!("\nNote: This preset shadows:");
        for shadowed in matches.iter().skip(1) {
            println!("  - {}", shadowed.display());
        }
    }

    Ok(())
}
