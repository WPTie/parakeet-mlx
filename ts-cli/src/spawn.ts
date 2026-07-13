import { execa, ExecaError } from "execa";
import { readFile } from "node:fs/promises";
import path from "node:path";
import type { CliOptions, TranscriptionResult } from "./types.js";

/**
 * Resolve the parakeet-mlx executable. Prefers `parakeet-mlx` on PATH, but
 * also accepts `awaz-mlx` as an alias.
 */
async function resolveExecutable(): Promise<string> {
  for (const candidate of ["parakeet-mlx", "awaz-mlx"]) {
    try {
      await execa("which", [candidate]);
      return candidate;
    } catch {
      // not found, try next
    }
  }
  throw new Error(
    "Neither parakeet-mlx nor awaz-mlx was found on PATH.\n" +
      "Install it first:\n\n" +
      "  uv tool install parakeet-mlx\n" +
      "  # or: pip install parakeet-mlx\n"
  );
}

/**
 * Build the argv list to pass to parakeet-mlx for a single audio file.
 * We always request JSON output so the TypeScript layer can reformat it.
 */
function buildArgs(audioFile: string, opts: CliOptions): string[] {
  const args: string[] = [
    audioFile,
    "--output-format",
    "json",
    "--output-dir",
    opts.outputDir,
    "--model",
    opts.model,
    "--decoding",
    opts.decoding,
    "--chunk-duration",
    String(opts.chunkDuration),
    "--overlap-duration",
    String(opts.overlapDuration),
    "--beam-size",
    String(opts.beamSize),
    "--length-penalty",
    String(opts.lengthPenalty),
    "--patience",
    String(opts.patience),
    "--duration-reward",
    String(opts.durationReward),
    "--local-attention-context-size",
    String(opts.localAttentionContextSize),
  ];

  if (opts.fp32) {
    args.push("--fp32");
  } else {
    args.push("--bf16");
  }

  if (opts.localAttention) {
    args.push("--local-attention");
  } else {
    args.push("--full-attention");
  }

  if (opts.maxWords !== undefined) {
    args.push("--max-words", String(opts.maxWords));
  }

  if (opts.silenceGap !== undefined) {
    args.push("--silence-gap", String(opts.silenceGap));
  }

  if (opts.maxDuration !== undefined) {
    args.push("--max-duration", String(opts.maxDuration));
  }

  if (opts.cacheDir !== undefined) {
    args.push("--cache-dir", opts.cacheDir);
  }

  if (opts.verbose) {
    args.push("--verbose");
  }

  return args;
}

/**
 * Build the output file stem from the template.
 * Supported placeholders: {parent}, {filename}, {index}, {date}
 */
export function buildStem(
  audioFile: string,
  _outputDir: string,
  template: string,
  index: number,
  date: Date
): string {
  const normalised = audioFile.replace(/\\/g, "/");
  const parts = normalised.split("/");
  const basename = parts[parts.length - 1] ?? audioFile;
  const dotIdx = basename.lastIndexOf(".");
  const filename = dotIdx !== -1 ? basename.slice(0, dotIdx) : basename;
  const parent = parts.length > 1 ? (parts[parts.length - 2] ?? "") : "";
  const dateStr = date.toISOString().slice(0, 10);

  return template
    .replace(/\{parent\}/g, parent)
    .replace(/\{filename\}/g, filename)
    .replace(/\{index\}/g, String(index))
    .replace(/\{date\}/g, dateStr);
}

/**
 * Parse the JSON output file written by parakeet-mlx for `audioFile`.
 * parakeet-mlx writes `{outputDir}/{stem}.json` (using the default template).
 */
async function readJsonResult(
  audioFile: string,
  outputDir: string,
  outputTemplate: string,
  fileIndex: number
): Promise<TranscriptionResult> {
  const stem = buildStem(audioFile, outputDir, outputTemplate, fileIndex, new Date());
  const jsonPath = path.join(outputDir, `${stem}.json`);
  const raw = await readFile(jsonPath, "utf8");
  return JSON.parse(raw) as TranscriptionResult;
}

/**
 * Run parakeet-mlx for a single audio file and return the parsed JSON result.
 * Progress/verbose output from the subprocess is forwarded to stderr.
 */
export async function transcribe(
  audioFile: string,
  opts: CliOptions,
  fileIndex: number
): Promise<TranscriptionResult> {
  const exe = await resolveExecutable();
  const args = buildArgs(audioFile, opts);

  if (opts.verbose) {
    process.stderr.write(`[awaz-mlx-ts] Running: ${exe} ${args.join(" ")}\n`);
  }

  try {
    const result = await execa(exe, args, {
      stderr: "inherit",
      // capture stdout so we can detect unexpected errors
      stdout: "pipe",
    });

    if (opts.verbose && result.stdout) {
      process.stderr.write(result.stdout + "\n");
    }
  } catch (err) {
    const execaErr = err as ExecaError;
    throw new Error(
      `parakeet-mlx failed for "${audioFile}": ${execaErr.message}`
    );
  }

  return readJsonResult(audioFile, opts.outputDir, opts.outputTemplate, fileIndex);
}

