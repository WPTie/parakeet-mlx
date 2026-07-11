use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

const SAMPLE_RATE: i32 = 16_000;

pub struct DecodedAudio {
    pub sample_rate: i32,
    pub samples: Vec<f32>,
}

pub fn decode(path: &Path) -> Result<DecodedAudio> {
    if !path.is_file() {
        bail!("audio file does not exist: {}", path.display());
    }

    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(path)
        .args([
            "-map", "0:a:0", "-ac", "1", "-ar", "16000", "-f", "f32le", "-",
        ])
        .stdin(Stdio::null())
        .output()
        .context("failed to start FFmpeg; install FFmpeg and ensure it is on PATH")?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        bail!(
            "FFmpeg could not decode {}: {}",
            path.display(),
            message.trim()
        );
    }
    if output.stdout.len() % 4 != 0 {
        bail!("FFmpeg returned malformed audio for {}", path.display());
    }

    let samples = output
        .stdout
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes(bytes.try_into().expect("four-byte chunk")))
        .collect::<Vec<_>>();
    if samples.is_empty() {
        bail!("audio file contains no samples: {}", path.display());
    }

    Ok(DecodedAudio {
        sample_rate: SAMPLE_RATE,
        samples,
    })
}
