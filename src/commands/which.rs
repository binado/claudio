use crate::preset::loader;
use anyhow::Result;

pub fn which(preset_name: &str) -> Result<()> {
    let matches = loader::find_all_presets(preset_name);
    for preset_match in matches {
        println!("{}", preset_match.display());
    }

    Ok(())
}
