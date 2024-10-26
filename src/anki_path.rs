use dirs::home_dir;
use std::path::PathBuf;

pub fn get_anki_media_directory() -> PathBuf {
    let mut path = home_dir().expect("failed to get home directory");

    if cfg!(target_os = "windows") {
        path.push("AppData/Roaming/Anki2");
    } else if cfg!(target_os = "macos") {
        path.push("Library/Application Support/Anki2");
    } else {
        path.push(".local/share/Anki2");
    }

    // TODO: add getting current user somehow
    path.push("User 1/collection.media");
    path
}
