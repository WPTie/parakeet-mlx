use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::Local;

use crate::cli::OutputFormat;
use crate::transcript::{Sentence, Transcript};

pub fn write_all(
    transcript: &Transcript,
    input: &Path,
    index: usize,
    output_dir: &Path,
    template: &str,
    format: OutputFormat,
    highlight_words: bool,
) -> Result<()> {
    let formats: &[OutputFormat] = match format {
        OutputFormat::All => &[
            OutputFormat::Txt,
            OutputFormat::Srt,
            OutputFormat::Vtt,
            OutputFormat::Json,
        ],
        _ => std::slice::from_ref(&format),
    };
    let basename = render_basename(template, input, index);

    for format in formats {
        let (extension, content) = match format {
            OutputFormat::Txt => ("txt", transcript.text.trim().to_owned()),
            OutputFormat::Srt => ("srt", to_srt(transcript, highlight_words)),
            OutputFormat::Vtt => ("vtt", to_vtt(transcript, highlight_words)),
            OutputFormat::Json => ("json", serde_json::to_string_pretty(transcript)?),
            OutputFormat::All => unreachable!("expanded above"),
        };
        let path = output_dir.join(format!("{basename}.{extension}"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("{extension}.tmp-{}", std::process::id()));
        fs::write(&temporary, content)
            .with_context(|| format!("failed to write {}", temporary.display()))?;
        fs::rename(&temporary, &path)
            .with_context(|| format!("failed to finalize {}", path.display()))?;
    }
    Ok(())
}

fn render_basename(template: &str, input: &Path, index: usize) -> String {
    let filename = input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("transcript");
    let parent = input
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .unwrap_or("");
    template
        .replace("{filename}", filename)
        .replace("{parent}", parent)
        .replace("{index}", &(index + 1).to_string())
        .replace("{date}", &Local::now().format("%Y%m%d").to_string())
}

fn to_srt(transcript: &Transcript, highlight_words: bool) -> String {
    subtitle_cues(transcript, highlight_words)
        .into_iter()
        .enumerate()
        .map(|(index, (start, end, text))| {
            format!(
                "{}\n{} --> {}\n{}\n",
                index + 1,
                timestamp(start, ','),
                timestamp(end, ','),
                text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_vtt(transcript: &Transcript, highlight_words: bool) -> String {
    let cues = subtitle_cues(transcript, highlight_words)
        .into_iter()
        .map(|(start, end, text)| {
            format!(
                "{} --> {}\n{}\n",
                timestamp(start, '.'),
                timestamp(end, '.'),
                text
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("WEBVTT\n\n{cues}")
}

fn subtitle_cues(transcript: &Transcript, highlight_words: bool) -> Vec<(f32, f32, String)> {
    if !highlight_words {
        return transcript
            .sentences
            .iter()
            .map(|sentence| (sentence.start, sentence.end, sentence.text.clone()))
            .collect();
    }

    transcript
        .sentences
        .iter()
        .flat_map(highlighted_sentence)
        .collect()
}

fn highlighted_sentence(sentence: &Sentence) -> Vec<(f32, f32, String)> {
    sentence
        .tokens
        .iter()
        .enumerate()
        .map(|(active, token)| {
            let text = sentence
                .tokens
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    if index == active {
                        format!("<u>{}</u>", value.text)
                    } else {
                        value.text.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            (token.start, token.end, text)
        })
        .collect()
}

fn timestamp(seconds: f32, marker: char) -> String {
    let millis = (seconds.max(0.0) * 1000.0).round() as u64;
    let hours = millis / 3_600_000;
    let minutes = (millis % 3_600_000) / 60_000;
    let seconds = (millis % 60_000) / 1_000;
    let millis = millis % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02}{marker}{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_timestamps_with_rounding() {
        assert_eq!(timestamp(3661.2346, ','), "01:01:01,235");
        assert_eq!(timestamp(-1.0, '.'), "00:00:00.000");
    }

    #[test]
    fn expands_safe_template_values() {
        let value = render_basename(
            "{parent}/{filename}-{index}-{date}",
            Path::new("audio/sample.wav"),
            1,
        );
        assert!(value.starts_with("audio/sample-2-"));
    }
}
