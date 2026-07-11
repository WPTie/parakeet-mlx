# awaz-mlx

Fast, native speech-to-text for Apple Silicon and desktop systems. `awaz-mlx`
is a Rust CLI powered by NVIDIA Parakeet Unified English 0.6B and ONNX Runtime.
It has no Python runtime, virtual environment, or NeMo installation.

## Features

- Native, statically linked Rust executable
- Latest unified English Parakeet 0.6B model
- Space-efficient INT8 inference
- Automatic, resumable-by-retry model cache
- WAV, MP3, M4A, FLAC, Ogg, Opus, and other FFmpeg-supported inputs
- Word timestamps and sentence segmentation
- TXT, SRT, WebVTT, and structured JSON output
- Bounded CPU threading and one model load for any number of input files

## Requirements

- macOS on Apple Silicon, Linux x86-64/aarch64, or Windows x86-64
- Rust 1.85 or newer when building from source
- FFmpeg available on `PATH`
- About 1 GB of free space for the quantized model cache

Install FFmpeg on macOS:

```console
brew install ffmpeg
```

## Install

```console
cargo install --path .
```

For an optimized local build:

```console
cargo build --release
./target/release/awaz-mlx --help
```

## Usage

The model is downloaded once on first use and then loaded from the operating
system's standard cache directory.

```console
# Generate an SRT subtitle
awaz-mlx meeting.m4a

# Transcribe several files and emit every format
awaz-mlx interview.wav podcast.mp3 --output-format all --output-dir transcripts

# Word-highlighted WebVTT with shorter cues
awaz-mlx talk.mp3 --output-format vtt --highlight-words --max-words 12

# Split cues around pauses and cap their duration
awaz-mlx lecture.flac --silence-gap 1.5 --max-duration 8

# Download the model ahead of time
awaz-mlx --download-model

# Use a separately managed model directory
awaz-mlx audio.wav --model-dir /models/parakeet-unified-int8
```

Run `awaz-mlx --help` for all options. `AWAZ_CACHE_DIR` overrides the default
model cache. A custom model directory must contain:

```text
encoder.int8.onnx
decoder.int8.onnx
joiner.int8.onnx
tokens.txt
```

## How it works

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                            awaz-mlx process                                 │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ CLI parser                                                                  │
│ • validates input paths and output settings                                 │
│ • selects CPU thread count                                                  │
│ • resolves managed or user-supplied model directory                         │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      │
                   cache miss ┌───────┴────────┐ cache hit
                              ▼                ▼
┌──────────────────────────────────────┐   ┌──────────────────────────────────┐
│ Model manager                        │   │ Existing model validation        │
│ HTTPS download → temporary archive   │   │ encoder + decoder + joiner       │
│ → bzip2 stream → safe tar extraction │   │ + token vocabulary               │
│ → atomic move into platform cache    │   └────────────────┬─────────────────┘
└───────────────────┬──────────────────┘                    │
                    └──────────────────┬─────────────────────┘
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ Native sherpa-onnx / ONNX Runtime recognizer                               │
│ • loads INT8 model once                                                     │
│ • creates an isolated inference stream for each input                       │
│ • uses a bounded CPU worker pool                                            │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      │
                    for each audio file│
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ FFmpeg decoder                                                              │
│ input container/codec → mono → 16 kHz → little-endian float32 PCM           │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ Parakeet Unified English 0.6B                                               │
│                                                                             │
│ PCM samples                                                                 │
│   │                                                                         │
│   ├─► 128-bin log-Mel feature extraction                                    │
│   │        │                                                                │
│   │        ▼                                                                │
│   ├─► Conformer encoder (24 layers, relative attention, 8× subsampling)     │
│   │        │                                                                │
│   │        ▼                                                                │
│   ├─► Transducer prediction network ◄── previous token                      │
│   │        │                                                                │
│   │        ▼                                                                │
│   └─► Joint network → token/duration probabilities → greedy decoding        │
│                                                                             │
│ output: text + BPE tokens + token timestamps + token durations              │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ Transcript normalizer                                                       │
│ • joins BPE pieces into timed words                                          │
│ • attaches punctuation                                                      │
│ • splits sentences on punctuation, silence, word count, or duration         │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ Output renderers                                                            │
│ TXT ─ plain text     SRT/VTT ─ subtitle cues     JSON ─ full timing tree    │
│                                                                             │
│ output template → output directory → atomic per-file write                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Performance

The release profile enables thin LTO, a single code-generation unit, and symbol
stripping. The model is loaded only once per command, audio conversion is done
in one FFmpeg process per file, and INT8 weights reduce memory bandwidth. Set
`--threads` to tune inference for your machine; the default uses up to eight
logical CPUs.

## Model and licenses

The source code is licensed under Apache-2.0.

The automatically downloaded model is
[NVIDIA Parakeet Unified English 0.6B](https://huggingface.co/nvidia/parakeet-unified-en-0.6b).
Its weights are separately licensed under
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). NVIDIA and NeMo are
the model authors; model use remains subject to the model card's terms,
limitations, and safety information.

Inference uses [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx), licensed
under Apache-2.0, and ONNX Runtime, licensed under MIT.
