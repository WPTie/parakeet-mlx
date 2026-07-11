# awaz-mlx

Apple Silicon speech-to-text using NVIDIA Parakeet and Apple's
[MLX](https://github.com/ml-explore/mlx) framework.

`awaz-mlx` exposes the complete CLI from
[senstella/parakeet-mlx](https://github.com/senstella/parakeet-mlx) 0.5.2 under
an additional command name. It uses MLX directly—there is no ONNX Runtime or
ONNX model conversion—and defaults to the same
[`mlx-community/parakeet-tdt-0.6b-v3`](https://huggingface.co/mlx-community/parakeet-tdt-0.6b-v3)
model.

## Requirements

- macOS on Apple Silicon
- Python 3.10 or newer
- FFmpeg available on `PATH`

```console
brew install ffmpeg
```

## Installation

Using [uv](https://docs.astral.sh/uv/):

```console
uv tool install .
```

Using pip:

```console
pip install .
```

The installation provides `awaz-mlx`. The upstream dependency also provides
the equivalent `parakeet-mlx` command.

## Usage

```console
awaz-mlx AUDIO_FILES... [OPTIONS]
```

The default output is an SRT transcript in the current directory.

### Options

- `--model` (default: `mlx-community/parakeet-tdt-0.6b-v3`, env:
  `PARAKEET_MODEL`) — Hugging Face model repository.
- `--output-dir` (default: current directory) — output directory.
- `--output-format` (default: `srt`, env: `PARAKEET_OUTPUT_FORMAT`) —
  `txt`, `srt`, `vtt`, `json`, or `all`.
- `--output-template` (default: `{filename}`, env:
  `PARAKEET_OUTPUT_TEMPLATE`) — supports `{parent}`, `{filename}`, `{index}`,
  and `{date}`.
- `--highlight-words` — add word-level highlighting and timestamps to SRT/VTT.
- `--verbose`, `-v` — print progress and debugging details.
- `--decoding` (default: `greedy`, env: `PARAKEET_DECODING`) — `greedy` or
  `beam`; beam decoding currently requires a TDT model.
- `--chunk-duration` (default: `120`, env: `PARAKEET_CHUNK_DURATION`) —
  long-audio chunk size in seconds; use `0` to disable.
- `--overlap-duration` (default: `15`, env: `PARAKEET_OVERLAP_DURATION`) —
  chunk overlap in seconds.
- `--beam-size` (default: `5`, env: `PARAKEET_BEAM_SIZE`).
- `--length-penalty` (default: `0.013`, env: `PARAKEET_LENGTH_PENALTY`).
- `--patience` (default: `3.5`, env: `PARAKEET_PATIENCE`).
- `--duration-reward` (default: `0.67`, env: `PARAKEET_DURATION_REWARD`).
- `--max-words` (env: `PARAKEET_MAX_WORDS`) — maximum words per sentence.
- `--silence-gap` (env: `PARAKEET_SILENCE_GAP`) — split sentences at a
  silence gap in seconds.
- `--max-duration` (env: `PARAKEET_MAX_DURATION`) — maximum sentence duration.
- `--fp32` / `--bf16` (default: `bf16`, env: `PARAKEET_FP32`) — inference
  precision.
- `--local-attention` / `--full-attention` (default: full attention, env:
  `PARAKEET_LOCAL_ATTENTION`) — use local attention to reduce intermediate
  memory use.
- `--local-attention-context-size` (default: `256`, env:
  `PARAKEET_LOCAL_ATTENTION_CTX`) — local attention window in frames.
- `--cache-dir` (env: `PARAKEET_CACHE_DIR`) — Hugging Face cache directory.
- `--version` — show the installed Parakeet MLX version.
- `--help` — show command help.

## Examples

```console
# Basic transcription
awaz-mlx audio.mp3

# Multiple files with word-level WebVTT timestamps
awaz-mlx *.mp3 --output-format vtt --highlight-words

# Beam decoding and all output formats
awaz-mlx audio.mp3 --decoding beam --beam-size 5 --output-format all

# Long audio without chunking, using local attention
awaz-mlx long.wav --chunk-duration 0 --local-attention
```

## Python API

Use the upstream `parakeet_mlx` module directly:

```python
from parakeet_mlx import from_pretrained

model = from_pretrained("mlx-community/parakeet-tdt-0.6b-v3")
result = model.transcribe("audio.wav")
print(result.text)
```

The upstream API also includes greedy and beam decoding, chunking, local
attention, sentence controls, streaming transcription, direct log-Mel input,
and TDT, RNNT, CTC, and TDT-CTC model variants.

## License

The wrapper is licensed under Apache-2.0. The upstream `parakeet-mlx`
dependency is also Apache-2.0. Model weights have their own license and usage
terms on Hugging Face.
