#!/usr/bin/env node
import { Command, InvalidArgumentError } from "commander";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { createRequire } from "node:module";
import { transcribe, buildStem } from "./spawn.js";
import { formatAll } from "./format.js";
import type { CliOptions, OutputFormat, DecodingMethod } from "./types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function parsePositiveFloat(value: string, flag: string): number {
  const n = parseFloat(value);
  if (isNaN(n)) throw new InvalidArgumentError(`${flag} must be a number.`);
  return n;
}

function parsePositiveInt(value: string, flag: string): number {
  const n = parseInt(value, 10);
  if (isNaN(n) || n < 1)
    throw new InvalidArgumentError(`${flag} must be a positive integer.`);
  return n;
}

function parseNonNegativeFloat(value: string, flag: string): number {
  const n = parseFloat(value);
  if (isNaN(n) || n < 0)
    throw new InvalidArgumentError(`${flag} must be >= 0.`);
  return n;
}

function packageVersion(): string {
  try {
    const req = createRequire(import.meta.url);
    // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
    return (req("../../package.json") as { version: string }).version;
  } catch {
    return "unknown";
  }
}

/** Extension for each output format. */
const FORMAT_EXTS: Record<Exclude<OutputFormat, "all">, string> = {
  txt: "txt",
  srt: "srt",
  vtt: "vtt",
  json: "json",
};

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  const program = new Command();

  program
    .name("awaz-mlx-ts")
    .description(
      "Apple Silicon speech-to-text using NVIDIA Parakeet and MLX.\n" +
        "Delegates inference to parakeet-mlx (Python/MLX) and handles all CLI\n" +
        "output formatting in TypeScript."
    )
    .version(packageVersion(), "--version", "Show version and exit")
    .argument("<audio_files...>", "One or more audio files to transcribe (WAV, MP3, FLAC, …)")
    // Model
    .option(
      "--model <repo>",
      "Hugging Face model repository",
      process.env["PARAKEET_MODEL"] ?? "mlx-community/parakeet-tdt-0.6b-v3"
    )
    // Output
    .option("--output-dir <dir>", "Directory to save outputs", ".")
    .option(
      "--output-format <fmt>",
      "Output format: txt, srt, vtt, json, all",
      (v: string) => {
        const allowed: OutputFormat[] = ["txt", "srt", "vtt", "json", "all"];
        if (!allowed.includes(v as OutputFormat))
          throw new InvalidArgumentError(
            `Invalid format "${v}". Choose from: ${allowed.join(", ")}`
          );
        return v as OutputFormat;
      },
      (process.env["PARAKEET_OUTPUT_FORMAT"] as OutputFormat | undefined) ?? "srt"
    )
    .option(
      "--output-template <tpl>",
      "Output filename template — supports {parent}, {filename}, {index}, {date}",
      process.env["PARAKEET_OUTPUT_TEMPLATE"] ?? "{filename}"
    )
    .option("--highlight-words", "Add word-level highlighting to SRT/VTT", false)
    .option("-v, --verbose", "Print progress and debugging details", false)
    // Decoding
    .option(
      "--decoding <method>",
      "Decoding method: greedy or beam (beam requires a TDT model)",
      (v: string) => {
        if (v !== "greedy" && v !== "beam")
          throw new InvalidArgumentError('Decoding must be "greedy" or "beam".');
        return v as DecodingMethod;
      },
      (process.env["PARAKEET_DECODING"] as DecodingMethod | undefined) ?? "greedy"
    )
    // Chunking
    .option(
      "--chunk-duration <sec>",
      "Long-audio chunk size in seconds; 0 to disable",
      (v) => parseNonNegativeFloat(v, "--chunk-duration"),
      process.env["PARAKEET_CHUNK_DURATION"]
        ? parseFloat(process.env["PARAKEET_CHUNK_DURATION"])
        : 120
    )
    .option(
      "--overlap-duration <sec>",
      "Chunk overlap in seconds",
      (v) => parseNonNegativeFloat(v, "--overlap-duration"),
      process.env["PARAKEET_OVERLAP_DURATION"]
        ? parseFloat(process.env["PARAKEET_OVERLAP_DURATION"])
        : 15
    )
    // Beam options
    .option(
      "--beam-size <n>",
      "Beam size (beam decoding only)",
      (v) => parsePositiveInt(v, "--beam-size"),
      process.env["PARAKEET_BEAM_SIZE"]
        ? parseInt(process.env["PARAKEET_BEAM_SIZE"], 10)
        : 5
    )
    .option(
      "--length-penalty <f>",
      "Length penalty; 0.0 to disable (beam decoding only)",
      (v) => parsePositiveFloat(v, "--length-penalty"),
      process.env["PARAKEET_LENGTH_PENALTY"]
        ? parseFloat(process.env["PARAKEET_LENGTH_PENALTY"])
        : 0.013
    )
    .option(
      "--patience <f>",
      "Beam patience; 1.0 to disable (beam decoding only)",
      (v) => parsePositiveFloat(v, "--patience"),
      process.env["PARAKEET_PATIENCE"]
        ? parseFloat(process.env["PARAKEET_PATIENCE"])
        : 3.5
    )
    .option(
      "--duration-reward <f>",
      "Duration reward 0–1 (TDT beam decoding only)",
      (v) => parsePositiveFloat(v, "--duration-reward"),
      process.env["PARAKEET_DURATION_REWARD"]
        ? parseFloat(process.env["PARAKEET_DURATION_REWARD"])
        : 0.67
    )
    // Sentence controls
    .option(
      "--max-words <n>",
      "Maximum words per sentence",
      (v) => parsePositiveInt(v, "--max-words"),
      process.env["PARAKEET_MAX_WORDS"]
        ? parseInt(process.env["PARAKEET_MAX_WORDS"], 10)
        : undefined
    )
    .option(
      "--silence-gap <sec>",
      "Split sentences at a silence gap of this many seconds",
      (v) => parsePositiveFloat(v, "--silence-gap"),
      process.env["PARAKEET_SILENCE_GAP"]
        ? parseFloat(process.env["PARAKEET_SILENCE_GAP"])
        : undefined
    )
    .option(
      "--max-duration <sec>",
      "Maximum sentence duration in seconds",
      (v) => parsePositiveFloat(v, "--max-duration"),
      process.env["PARAKEET_MAX_DURATION"]
        ? parseFloat(process.env["PARAKEET_MAX_DURATION"])
        : undefined
    )
    // Precision
    .option(
      "--fp32",
      "Use FP32 precision (default: bf16)",
      process.env["PARAKEET_FP32"] === "1" || process.env["PARAKEET_FP32"] === "true"
    )
    .option("--bf16", "Use BF16 precision (default, ignored if --fp32 given)")
    // Attention
    .option(
      "--local-attention",
      "Use local attention (reduces intermediate memory; good for long audio without chunking)",
      process.env["PARAKEET_LOCAL_ATTENTION"] === "1" ||
        process.env["PARAKEET_LOCAL_ATTENTION"] === "true"
    )
    .option("--full-attention", "Use full attention (default)")
    .option(
      "--local-attention-context-size <n>",
      "Local attention window in frames",
      (v) => parsePositiveInt(v, "--local-attention-context-size"),
      process.env["PARAKEET_LOCAL_ATTENTION_CTX"]
        ? parseInt(process.env["PARAKEET_LOCAL_ATTENTION_CTX"], 10)
        : 256
    )
    // Cache
    .option(
      "--cache-dir <dir>",
      "Hugging Face cache directory",
      process.env["PARAKEET_CACHE_DIR"]
    );

  program.parse();

  const rawOpts = program.opts<{
    model: string;
    outputDir: string;
    outputFormat: OutputFormat;
    outputTemplate: string;
    highlightWords: boolean;
    verbose: boolean;
    decoding: DecodingMethod;
    chunkDuration: number;
    overlapDuration: number;
    beamSize: number;
    lengthPenalty: number;
    patience: number;
    durationReward: number;
    maxWords: number | undefined;
    silenceGap: number | undefined;
    maxDuration: number | undefined;
    fp32: boolean;
    bf16: boolean;
    localAttention: boolean;
    fullAttention: boolean;
    localAttentionContextSize: number;
    cacheDir: string | undefined;
  }>();

  const audioFiles = program.args as string[];

  if (audioFiles.length === 0) {
    program.help();
  }

  // Resolve --fp32 / --bf16 (--fp32 wins if both given)
  const useFp32 = rawOpts.fp32 === true;
  // Resolve --local-attention / --full-attention (--local-attention wins if both given)
  const useLocalAttention = rawOpts.localAttention === true;

  const opts: CliOptions = {
    audioFiles,
    model: rawOpts.model,
    outputDir: rawOpts.outputDir,
    outputFormat: rawOpts.outputFormat,
    outputTemplate: rawOpts.outputTemplate,
    highlightWords: rawOpts.highlightWords,
    verbose: rawOpts.verbose,
    decoding: rawOpts.decoding,
    chunkDuration: rawOpts.chunkDuration,
    overlapDuration: rawOpts.overlapDuration,
    beamSize: rawOpts.beamSize,
    lengthPenalty: rawOpts.lengthPenalty,
    patience: rawOpts.patience,
    durationReward: rawOpts.durationReward,
    maxWords: rawOpts.maxWords,
    silenceGap: rawOpts.silenceGap,
    maxDuration: rawOpts.maxDuration,
    fp32: useFp32,
    localAttention: useLocalAttention,
    localAttentionContextSize: rawOpts.localAttentionContextSize,
    cacheDir: rawOpts.cacheDir,
  };

  // Ensure output directory exists
  await mkdir(opts.outputDir, { recursive: true });

  const now = new Date();
  let hasError = false;

  for (let i = 0; i < audioFiles.length; i++) {
    const audioFile = audioFiles[i]!;
    if (opts.verbose) {
      process.stderr.write(
        `\n[awaz-mlx-ts] Transcribing ${i + 1}/${audioFiles.length}: ${audioFile}\n`
      );
    }

    try {
      const result = await transcribe(audioFile, opts, i, now);
      const formatted = formatAll(result, opts.outputFormat, opts.highlightWords);
      const stem = buildStem(audioFile, opts.outputDir, opts.outputTemplate, i, now);

      for (const [fmt, content] of formatted) {
        const ext = FORMAT_EXTS[fmt as Exclude<OutputFormat, "all">];
        const outPath = path.join(opts.outputDir, `${stem}.${ext}`);
        await writeFile(outPath, content, "utf8");
        process.stdout.write(`${outPath}\n`);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      process.stderr.write(`[awaz-mlx-ts] Error: ${msg}\n`);
      hasError = true;
    }
  }

  if (hasError) {
    process.exit(1);
  }
}

main().catch((err: unknown) => {
  const msg = err instanceof Error ? err.message : String(err);
  process.stderr.write(`[awaz-mlx-ts] Fatal: ${msg}\n`);
  process.exit(1);
});
