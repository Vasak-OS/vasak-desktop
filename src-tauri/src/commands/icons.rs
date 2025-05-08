use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use gtk::prelude::IconThemeExt;
use std::fs;

#[tauri::command]
pub fn get_icon_base64(name: &str) -> Result<String, String> {
    let themed = gtk::IconTheme::default().unwrap();
    let mut themed_icon = themed.lookup_icon(
        name,
        64,
        gtk::IconLookupFlags::FORCE_SVG | gtk::IconLookupFlags::FORCE_REGULAR,
    );

    if themed_icon == None {
        themed_icon = themed.lookup_icon(
            "image-missing",
            64,
            gtk::IconLookupFlags::FORCE_SVG | gtk::IconLookupFlags::FORCE_REGULAR,
        );
    }

    let icon = themed_icon.unwrap().filename().unwrap();

    let icon_data = fs::read(icon).map_err(|e| e.to_string())?;
    Ok(STANDARD.encode(icon_data))
}

#[tauri::command]
pub fn get_symbol_base64(name: &str) -> Result<String, String> {
    let themed = gtk::IconTheme::default().unwrap();

    let mut themed_icon = themed
        .lookup_icon(
            name,
            64,
            gtk::IconLookupFlags::FORCE_SYMBOLIC | gtk::IconLookupFlags::FORCE_SVG,
        );

    if themed_icon == None {
        themed_icon = themed.lookup_icon(
            "image-missing",
            64,
            gtk::IconLookupFlags::FORCE_SYMBOLIC | gtk::IconLookupFlags::FORCE_SVG,
        );
    }

    let icon = themed_icon.unwrap().filename().unwrap();

    let icon_data = fs::read(icon).map_err(|e| e.to_string())?;
    Ok(STANDARD.encode(icon_data))
}
