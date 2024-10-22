use std::fs;
use std::path::Path;

use tokio::process::Command;

pub async fn normalize_audio_file(path: &Path) {
    let input_path = path.to_str().expect("invalid path");
    let temp_output_path = format!(
        "./temp_normalized_{}",
        path.file_name().unwrap().to_str().unwrap()
    );

    log::debug!("normalizing {} to {}", input_path, temp_output_path);

    let supported_extensions = ["mp3", "wav", "flac", "aac", "ogg"];
    assert!(supported_extensions
        .iter()
        .any(|&extension| input_path.ends_with(extension)));

    let output = Command::new("ffmpeg")
        .args(&[
            "-i",
            input_path,
            "-af",
            "speechnorm=e=6.25:r=0.00001:l=1, loudnorm=I=-16:TP=-2:LRA=11",
            "-y",
            &temp_output_path,
        ])
        .output()
        .await
        .expect("failed to execute ffmpeg");

    if output.status.success() {
        fs::rename(temp_output_path.clone(), input_path)
            .expect("failed to rename temp file to original");
        fs::copy(input_path, temp_output_path.clone()).unwrap();
        log::info!("completed audio normalization, output file: {}", input_path);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("failed to perform normalization: {}", stderr);

        // let _ = fs::remove_file(temp_output_path);
    }
}
