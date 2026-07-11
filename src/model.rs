use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use bzip2::read::BzDecoder;
use directories::ProjectDirs;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};

const MODEL_NAME: &str = "parakeet-unified-en-0.6b-int8";
const ARCHIVE_ROOT: &str = "sherpa-onnx-nemo-parakeet-unified-en-0.6b-int8-non-streaming";
const MODEL_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-unified-en-0.6b-int8-non-streaming.tar.bz2";
const MODEL_SHA256: &str = "99f63605b3a85a54c250c0869670a687b7d6598a47bf2421515e1f839a76e150";

pub struct ModelFiles {
    pub directory: PathBuf,
    pub encoder: PathBuf,
    pub decoder: PathBuf,
    pub joiner: PathBuf,
    pub tokens: PathBuf,
}

impl ModelFiles {
    pub fn resolve(
        supplied: Option<&Path>,
        cache_override: Option<&Path>,
        force_download: bool,
    ) -> Result<Self> {
        if let Some(directory) = supplied {
            return Self::from_directory(directory.to_owned());
        }

        let cache_root = match cache_override {
            Some(path) => path.to_owned(),
            None => ProjectDirs::from("", "", "awaz-mlx")
                .context("could not determine the platform cache directory")?
                .cache_dir()
                .to_owned(),
        };
        let directory = cache_root.join(MODEL_NAME);
        if force_download && directory.exists() {
            fs::remove_dir_all(&directory)
                .with_context(|| format!("failed to remove {}", directory.display()))?;
        }
        if !directory.exists() {
            download(&cache_root, &directory)?;
        }
        Self::from_directory(directory)
    }

    fn from_directory(directory: PathBuf) -> Result<Self> {
        let model = Self {
            encoder: directory.join("encoder.int8.onnx"),
            decoder: directory.join("decoder.int8.onnx"),
            joiner: directory.join("joiner.int8.onnx"),
            tokens: directory.join("tokens.txt"),
            directory,
        };
        for path in [&model.encoder, &model.decoder, &model.joiner, &model.tokens] {
            if !path.is_file() {
                bail!("model is incomplete; missing {}", path.display());
            }
        }
        Ok(model)
    }
}

fn download(cache_root: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(cache_root)
        .with_context(|| format!("failed to create {}", cache_root.display()))?;
    let staging = cache_root.join(format!(".download-{}", std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let result = download_into(&staging, destination);
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn download_into(staging: &Path, destination: &Path) -> Result<()> {
    eprintln!("Downloading NVIDIA Parakeet Unified English 0.6B (INT8)...");
    let archive_path = staging.join("model.tar.bz2");
    let mut response = ureq::get(MODEL_URL)
        .call()
        .context("failed to download the Parakeet model")?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let progress = total.map(|length| {
        let bar = ProgressBar::new(length);
        bar.set_style(
            ProgressStyle::with_template(
                "{spinner:.cyan} [{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})",
            )
            .expect("valid progress template"),
        );
        bar
    });

    let mut source = response.body_mut().as_reader();
    let mut target = fs::File::create(&archive_path)?;
    let mut checksum = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        target.write_all(&buffer[..count])?;
        checksum.update(&buffer[..count]);
        if let Some(bar) = &progress {
            bar.inc(count as u64);
        }
    }
    if let Some(bar) = progress {
        bar.finish_and_clear();
    }
    target.sync_all()?;
    let actual_checksum = checksum
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual_checksum != MODEL_SHA256 {
        bail!("model checksum mismatch: expected {MODEL_SHA256}, received {actual_checksum}");
    }

    let archive = fs::File::open(&archive_path)?;
    tar::Archive::new(BzDecoder::new(archive))
        .unpack(staging)
        .context("failed to extract the model archive")?;
    let extracted = staging.join(ARCHIVE_ROOT);
    ModelFiles::from_directory(extracted.clone())?;
    fs::rename(&extracted, destination)
        .with_context(|| format!("failed to install model in {}", destination.display()))?;
    fs::remove_dir_all(staging)?;
    Ok(())
}
