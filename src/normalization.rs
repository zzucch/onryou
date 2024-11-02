use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context};
use tokio::process::Command;

pub async fn normalize_audio_file(path: &Path) -> anyhow::Result<()> {
    let input_path = path.to_str().with_context(|| "invalid path")?;
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow!("failed to retrieve filename from path"))?
        .to_str()
        .ok_or_else(|| anyhow!("failed to convert filename to string"))?;

    let temporary_output_path = format!("./tmp_normalized_{filename}");

    log::debug!("normalizing {} to {}", input_path, temporary_output_path);

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
        .with_context(|| "failed to execute ffmpeg")?;

    if output.status.success() {
        fs::rename(temporary_output_path.clone(), input_path)
            .with_context(|| "failed to rename temp file to original")?;
        log::info!("completed audio normalization, output file: {}", input_path);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("failed to perform normalization: {}", stderr);

        let _ = fs::remove_file(temporary_output_path);
    }

    Ok(())
}
