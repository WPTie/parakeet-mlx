# awaz-mlx-ts

TypeScript CLI rewrite of [senstella/parakeet-mlx](https://github.com/senstella/parakeet-mlx).
Full feature parity, same MLX model, Node.js runtime.

## How it works

`awaz-mlx-ts` is a TypeScript/Node.js CLI that delegates model inference to
the Python `parakeet-mlx` library (Apple MLX) and handles all CLI argument
parsing, validation, output formatting (txt/srt/vtt/json/all), and file writing
itself. The inference engine is always Python + MLX, so the same
`mlx-community/parakeet-tdt-0.6b-v3` model weights are used.

```
awaz-mlx-ts audio.mp3
    │
    ├── parses & validates args (TypeScript)
    ├── spawns: parakeet-mlx audio.mp3 --output-format json …
    │              └── Python + MLX inference
    ├── reads JSON result
    └── writes SRT / VTT / txt / json / all (TypeScript)
```

## Requirements

- macOS on Apple Silicon
- Node.js 18 or newer
- `parakeet-mlx` (or `awaz-mlx`) installed and on `PATH`:
  ```console
  uv tool install parakeet-mlx
  # or: pip install parakeet-mlx
  ```
- FFmpeg available on `PATH`:
  ```console
  brew install ffmpeg
  ```

## Installation

```console
cd ts-cli
npm install
npm run build
node dist/cli.js --help
```

To install globally:

```console
npm link
awaz-mlx-ts --help
```

## Usage

```console
awaz-mlx-ts AUDIO_FILES... [OPTIONS]
```

The default output is an SRT transcript in the current directory.

### Options

All environment-variable defaults match the upstream `parakeet-mlx` variables.

| Flag | Default | Env | Description |
|---|---|---|---|
| `--model <repo>` | `mlx-community/parakeet-tdt-0.6b-v3` | `PARAKEET_MODEL` | Hugging Face model repository |
| `--output-dir <dir>` | `.` | — | Directory to save outputs |
| `--output-format <fmt>` | `srt` | `PARAKEET_OUTPUT_FORMAT` | `txt`, `srt`, `vtt`, `json`, or `all` |
| `--output-template <tpl>` | `{filename}` | `PARAKEET_OUTPUT_TEMPLATE` | Filename template; supports `{parent}`, `{filename}`, `{index}`, `{date}` |
| `--highlight-words` | `false` | — | Word-level highlighting in SRT/VTT |
| `-v, --verbose` | `false` | — | Print progress and debug info |
| `--decoding <method>` | `greedy` | `PARAKEET_DECODING` | `greedy` or `beam` (beam requires TDT model) |
| `--chunk-duration <sec>` | `120` | `PARAKEET_CHUNK_DURATION` | Long-audio chunk size; `0` to disable |
| `--overlap-duration <sec>` | `15` | `PARAKEET_OVERLAP_DURATION` | Chunk overlap in seconds |
| `--beam-size <n>` | `5` | `PARAKEET_BEAM_SIZE` | Beam size (beam only) |
| `--length-penalty <f>` | `0.013` | `PARAKEET_LENGTH_PENALTY` | Length penalty; `0.0` to disable (beam only) |
| `--patience <f>` | `3.5` | `PARAKEET_PATIENCE` | Beam patience; `1.0` to disable (beam only) |
| `--duration-reward <f>` | `0.67` | `PARAKEET_DURATION_REWARD` | Duration reward 0–1 (TDT beam only) |
| `--max-words <n>` | — | `PARAKEET_MAX_WORDS` | Maximum words per sentence |
| `--silence-gap <sec>` | — | `PARAKEET_SILENCE_GAP` | Split at silence gap (seconds) |
| `--max-duration <sec>` | — | `PARAKEET_MAX_DURATION` | Maximum sentence duration |
| `--fp32` / `--bf16` | bf16 | `PARAKEET_FP32` | Inference precision |
| `--local-attention` / `--full-attention` | full | `PARAKEET_LOCAL_ATTENTION` | Attention mode |
| `--local-attention-context-size <n>` | `256` | `PARAKEET_LOCAL_ATTENTION_CTX` | Local attention window (frames) |
| `--cache-dir <dir>` | — | `PARAKEET_CACHE_DIR` | Hugging Face cache directory |
| `--version` | — | — | Show version |
| `-h, --help` | — | — | Show help |

## Examples

```console
# Basic transcription
awaz-mlx-ts audio.mp3

# Multiple files with word-level WebVTT timestamps
awaz-mlx-ts *.mp3 --output-format vtt --highlight-words

# Beam decoding, all output formats
awaz-mlx-ts audio.mp3 --decoding beam --beam-size 5 --output-format all

# Long audio without chunking, using local attention
awaz-mlx-ts long.wav --chunk-duration 0 --local-attention

# Custom output directory and filename template
awaz-mlx-ts *.mp3 --output-dir transcripts/ --output-template "{date}-{filename}"
```

## Development

```console
cd ts-cli
npm install
npm run typecheck   # tsc --noEmit
npm run build       # tsc → dist/
```

## License

Apache-2.0
