use serde::Serialize;
use sherpa_onnx::OfflineRecognizerResult;

#[derive(Debug, Clone, Copy)]
pub struct SentenceOptions {
    pub max_words: Option<usize>,
    pub silence_gap: Option<f32>,
    pub max_duration: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Token {
    pub text: String,
    pub start: f32,
    pub end: f32,
    pub duration: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sentence {
    pub text: String,
    pub start: f32,
    pub end: f32,
    pub duration: f32,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Transcript {
    pub text: String,
    pub duration: f32,
    pub sentences: Vec<Sentence>,
}

impl Transcript {
    pub fn from_recognizer(
        result: OfflineRecognizerResult,
        audio_duration: f32,
        options: SentenceOptions,
    ) -> Self {
        let tokens = aligned_tokens(
            result.tokens,
            result.timestamps.unwrap_or_default(),
            result.durations.unwrap_or_default(),
            audio_duration,
        );
        let sentences = split_sentences(tokens, options);
        Self {
            text: result.text.trim().to_owned(),
            duration: audio_duration,
            sentences,
        }
    }
}

fn aligned_tokens(
    raw: Vec<String>,
    timestamps: Vec<f32>,
    durations: Vec<f32>,
    audio_duration: f32,
) -> Vec<Token> {
    let mut output: Vec<Token> = Vec::new();
    for (index, raw_text) in raw.into_iter().enumerate() {
        let starts_word = raw_text.starts_with('▁') || raw_text.starts_with(' ');
        let text = raw_text.trim_start_matches(['▁', ' ']).to_owned();
        if text.is_empty() {
            continue;
        }
        let start = timestamps.get(index).copied().unwrap_or_else(|| {
            output.last().map(|token| token.end).unwrap_or(0.0)
        });
        let end = durations
            .get(index)
            .map(|duration| start + duration)
            .or_else(|| timestamps.get(index + 1).copied())
            .unwrap_or(audio_duration)
            .clamp(start, audio_duration.max(start));

        if !starts_word && !output.is_empty() && !is_standalone_punctuation(&text) {
            let previous = output.last_mut().expect("checked as non-empty");
            previous.text.push_str(&text);
            previous.end = end;
            previous.duration = previous.end - previous.start;
        } else if is_standalone_punctuation(&text) && !output.is_empty() {
            let previous = output.last_mut().expect("checked as non-empty");
            previous.text.push_str(&text);
            previous.end = end;
            previous.duration = previous.end - previous.start;
        } else {
            output.push(Token {
                text,
                start,
                end,
                duration: end - start,
            });
        }
    }
    output
}

fn split_sentences(tokens: Vec<Token>, options: SentenceOptions) -> Vec<Sentence> {
    let mut sentences = Vec::new();
    let mut current = Vec::new();
    for token in tokens {
        let previous_end = current.last().map(|value: &Token| value.end);
        let silence_split = previous_end
            .zip(options.silence_gap)
            .is_some_and(|(end, gap)| token.start - end >= gap);
        if silence_split {
            push_sentence(&mut sentences, std::mem::take(&mut current));
        }

        current.push(token);
        let last = current.last().expect("token was just inserted");
        let punctuation_split = last
            .text
            .chars()
            .last()
            .is_some_and(|character| ".!?。？！".contains(character));
        let words_split = options
            .max_words
            .is_some_and(|limit| current.len() >= limit);
        let duration_split = options
            .max_duration
            .is_some_and(|limit| last.end - current[0].start >= limit);
        if punctuation_split || words_split || duration_split {
            push_sentence(&mut sentences, std::mem::take(&mut current));
        }
    }
    push_sentence(&mut sentences, current);
    sentences
}

fn push_sentence(sentences: &mut Vec<Sentence>, tokens: Vec<Token>) {
    let (Some(first), Some(last)) = (tokens.first(), tokens.last()) else {
        return;
    };
    let start = first.start;
    let end = last.end;
    let text = join_words(&tokens);
    sentences.push(Sentence {
        text,
        start,
        end,
        duration: end - start,
        tokens,
    });
}

fn join_words(tokens: &[Token]) -> String {
    let mut text = String::new();
    for token in tokens {
        if !text.is_empty() && !token.text.chars().all(|character| ",.!?;:。？！".contains(character)) {
            text.push(' ');
        }
        text.push_str(&token.text);
    }
    text
}

fn is_standalone_punctuation(text: &str) -> bool {
    text.chars()
        .all(|character| ",.!?;:。？！'’".contains(character))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(text: &str, start: f32, end: f32) -> Token {
        Token {
            text: text.into(),
            start,
            end,
            duration: end - start,
        }
    }

    #[test]
    fn splits_on_punctuation_and_word_limit() {
        let tokens = vec![
            token("Hello", 0.0, 0.4),
            token("world.", 0.4, 0.8),
            token("One", 1.0, 1.2),
            token("two", 1.2, 1.5),
        ];
        let sentences = split_sentences(
            tokens,
            SentenceOptions {
                max_words: Some(2),
                silence_gap: None,
                max_duration: None,
            },
        );
        assert_eq!(sentences.len(), 2);
        assert_eq!(sentences[0].text, "Hello world.");
        assert_eq!(sentences[1].text, "One two");
    }

    #[test]
    fn splits_before_word_after_silence() {
        let sentences = split_sentences(
            vec![token("first", 0.0, 0.4), token("second", 2.0, 2.4)],
            SentenceOptions {
                max_words: None,
                silence_gap: Some(1.0),
                max_duration: None,
            },
        );
        assert_eq!(sentences.len(), 2);
    }
}
