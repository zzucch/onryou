use std::fs;
use std::path::Path;

use tokio::process::Command;

pub async fn normalize_audio_file(path: &Path) {
    let input_path = path.to_str().expect("invalid path");
    let temporary_output_path = format!(
        "./tmp_normalized_{}",
        path.file_name().unwrap().to_str().unwrap()
    );

    log::debug!("normalizing {} to {}", input_path, temporary_output_path);

    let supported_extensions = ["mp3", "wav", "flac", "aac", "ogg"];
    assert!(supported_extensions
        .iter()
        .any(|&extension| input_path.ends_with(extension)));

    let output = Command::new("ffmpeg")
        .args([
            "-i",
            input_path,
            "-af",
            "speechnorm=e=6.25:r=0.00001:l=1, loudnorm=I=-16:TP=-2:LRA=11",
            "-y",
            &temporary_output_path,
        ])
        .output()
        .await
        .expect("failed to execute ffmpeg");

    if output.status.success() {
        fs::rename(temporary_output_path.clone(), input_path)
            .expect("failed to rename temp file to original");
        log::info!("completed audio normalization, output file: {}", input_path);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("failed to perform normalization: {}", stderr);

        let _ = fs::remove_file(temporary_output_path);
    }
}
