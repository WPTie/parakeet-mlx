use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Txt,
    Srt,
    Vtt,
    Json,
    All,
}

#[derive(Debug, Parser)]
#[command(
    name = "awaz-mlx",
    version,
    about = "Native NVIDIA Parakeet speech recognition",
    after_help = "Model: NVIDIA Parakeet Unified English 0.6B (INT8, cached automatically)"
)]
pub struct Cli {
    /// Audio files to transcribe (any format supported by FFmpeg)
    #[arg(value_name = "AUDIO")]
    pub audio_files: Vec<PathBuf>,

    /// Download the model, print its directory, and exit
    #[arg(long)]
    pub download_model: bool,

    /// Use model files from this directory instead of the managed cache
    #[arg(long, value_name = "DIR")]
    pub model_dir: Option<PathBuf>,

    /// Override the managed model cache directory
    #[arg(long, env = "AWAZ_CACHE_DIR", value_name = "DIR")]
    pub cache_dir: Option<PathBuf>,

    /// Re-download the managed model
    #[arg(long)]
    pub force_download: bool,

    /// Directory for generated transcripts
    #[arg(short, long, default_value = ".", value_name = "DIR")]
    pub output_dir: PathBuf,

    /// Transcript format
    #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Srt)]
    pub output_format: OutputFormat,

    /// Output basename template: {filename}, {parent}, {index}, {date}
    #[arg(long, default_value = "{filename}")]
    pub output_template: String,

    /// Emit one highlighted subtitle cue per word
    #[arg(long)]
    pub highlight_words: bool,

    /// Maximum words per subtitle sentence
    #[arg(long)]
    pub max_words: Option<usize>,

    /// Split after this many seconds of silence
    #[arg(long)]
    pub silence_gap: Option<f32>,

    /// Maximum subtitle sentence duration in seconds
    #[arg(long)]
    pub max_duration: Option<f32>,

    /// CPU inference threads
    #[arg(
        short = 'j',
        long,
        default_value_t = default_threads(),
        value_parser = clap::value_parser!(i32).range(1..=256)
    )]
    pub threads: i32,

    /// Print model and performance details
    #[arg(short, long)]
    pub verbose: bool,
}

impl Cli {
    pub fn validate(&self) -> Result<()> {
        if self.audio_files.is_empty() && !self.download_model {
            bail!("provide at least one audio file, or use --download-model");
        }
        if self.model_dir.is_some() && self.force_download {
            bail!("--force-download cannot be combined with --model-dir");
        }
        if self.max_words == Some(0) {
            bail!("--max-words must be greater than zero");
        }
        if self.silence_gap.is_some_and(|value| value <= 0.0)
            || self.max_duration.is_some_and(|value| value <= 0.0)
        {
            bail!("duration options must be greater than zero");
        }
        Ok(())
    }
}

fn default_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|count| count.get().min(8) as i32)
        .unwrap_or(4)
}
