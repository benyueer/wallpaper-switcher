use crate::state::AppState;

pub fn apply_wallpaper(state: &AppState) {
    if let Some(path) = state.current_path() {
        if let Some(p) = path.to_str() {
            let _ = wallpaper::set_mode(wallpaper::Mode::Fit);
            if let Err(e) = wallpaper::set_from_path(p) {
                eprintln!("Failed to set wallpaper: {:?}", e);
            }
        }
    }
}
