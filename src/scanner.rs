use walkdir::WalkDir;
use std::path::PathBuf;

pub fn load_images_from_dirs(dirs: &[String]) -> Vec<PathBuf> {
    let exts = ["jpg", "jpeg", "png", "bmp"];

    let mut result = Vec::new();

    for dir in dirs {
        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if exts.contains(&ext.as_str()) {
                    result.push(path.to_path_buf());
                }
            }
        }
    }

    result
}