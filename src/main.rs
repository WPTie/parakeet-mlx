mod audio;
mod cli;
mod model;
mod output;
mod transcript;

use std::fs;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig};

use crate::cli::{Cli, OutputFormat};
use crate::model::ModelFiles;
use crate::transcript::{SentenceOptions, Transcript};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    cli.validate()?;

    let model = ModelFiles::resolve(
        cli.model_dir.as_deref(),
        cli.cache_dir.as_deref(),
        cli.force_download,
    )?;
    if cli.download_model {
        println!("{}", model.directory.display());
        return Ok(());
    }

    let recognizer = create_recognizer(&model, cli.threads, cli.verbose)?;
    fs::create_dir_all(&cli.output_dir)
        .with_context(|| format!("failed to create {}", cli.output_dir.display()))?;

    let progress = if cli.verbose {
        ProgressBar::hidden()
    } else {
        let bar = ProgressBar::new(cli.audio_files.len() as u64);
        bar.set_style(
            ProgressStyle::with_template(
                "{spinner:.cyan} [{elapsed_precise}] {wide_msg} {pos}/{len}",
            )
            .expect("valid progress template"),
        );
        bar
    };

    let options = SentenceOptions {
        max_words: cli.max_words,
        silence_gap: cli.silence_gap,
        max_duration: cli.max_duration,
    };
    let started = Instant::now();

    for (index, path) in cli.audio_files.iter().enumerate() {
        progress.set_message(format!("Transcribing {}", path.display()));
        let decoded = audio::decode(path)?;
        let audio_duration = decoded.samples.len() as f32 / decoded.sample_rate as f32;
        let stream = recognizer.create_stream();
        stream.accept_waveform(decoded.sample_rate, &decoded.samples);

        let inference_started = Instant::now();
        recognizer.decode(&stream);
        let raw = stream
            .get_result()
            .with_context(|| format!("model returned no result for {}", path.display()))?;
        let inference_time = inference_started.elapsed().as_secs_f32();
        let transcript = Transcript::from_recognizer(raw, audio_duration, options);

        output::write_all(
            &transcript,
            path,
            index,
            &cli.output_dir,
            &cli.output_template,
            cli.output_format,
            cli.highlight_words,
        )?;

        if cli.verbose {
            let rtf = if audio_duration > 0.0 {
                inference_time / audio_duration
            } else {
                0.0
            };
            println!(
                "{} ({audio_duration:.1}s audio, {inference_time:.2}s inference, {rtf:.3} RTF)\n{}",
                path.display(),
                transcript.text
            );
        }
        progress.inc(1);
    }

    progress.finish_with_message(format!(
        "Completed {} file(s) in {:.2}s",
        cli.audio_files.len(),
        started.elapsed().as_secs_f32()
    ));

    if cli.output_format == OutputFormat::Json && cli.verbose {
        eprintln!("Structured timestamps were written as JSON.");
    }
    Ok(())
}

fn create_recognizer(model: &ModelFiles, threads: i32, verbose: bool) -> Result<OfflineRecognizer> {
    let mut config = OfflineRecognizerConfig::default();
    config.model_config.transducer = OfflineTransducerModelConfig {
        encoder: Some(model.encoder.to_string_lossy().into_owned()),
        decoder: Some(model.decoder.to_string_lossy().into_owned()),
        joiner: Some(model.joiner.to_string_lossy().into_owned()),
    };
    config.model_config.tokens = Some(model.tokens.to_string_lossy().into_owned());
    config.model_config.provider = Some("cpu".into());
    config.model_config.num_threads = threads;
    config.model_config.debug = verbose;
    config.decoding_method = Some("greedy_search".into());

    OfflineRecognizer::create(&config)
        .ok_or_else(|| anyhow::anyhow!("failed to initialize Parakeet"))
}
