/**
 * A single word-level token with timing information from parakeet-mlx JSON output.
 */
export interface AlignedToken {
  text: string;
  start: number;
  end: number;
  duration: number;
}

/**
 * A sentence-level segment with timing information from parakeet-mlx JSON output.
 */
export interface AlignedSentence {
  text: string;
  start: number;
  end: number;
  duration: number;
  tokens: AlignedToken[];
}

/**
 * Full transcription result from parakeet-mlx `--output-format json`.
 */
export interface TranscriptionResult {
  text: string;
  sentences: AlignedSentence[];
}

/**
 * Supported output formats.
 */
export type OutputFormat = "txt" | "srt" | "vtt" | "json" | "all";

/**
 * Decoding methods.
 */
export type DecodingMethod = "greedy" | "beam";

/**
 * All CLI options collected after parsing.
 */
export interface CliOptions {
  audioFiles: string[];
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
  localAttention: boolean;
  localAttentionContextSize: number;
  cacheDir: string | undefined;
}

/**
 * Per-file output produced by the formatter.
 */
export interface FormattedOutput {
  /** Base name (no extension) derived from the template. */
  basename: string;
  /** Map of format → file content string. */
  files: Map<OutputFormat, string>;
}
