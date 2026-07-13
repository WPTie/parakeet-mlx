import type { AlignedSentence, AlignedToken, TranscriptionResult, OutputFormat } from "./types.js";

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/**
 * Convert a fractional number of seconds to an SRT timestamp string:
 * `HH:MM:SS,mmm`
 */
function toSrtTimestamp(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  const ms = Math.round((seconds - Math.floor(seconds)) * 1000);
  return `${pad2(h)}:${pad2(m)}:${pad2(s)},${pad3(ms)}`;
}

/**
 * Convert a fractional number of seconds to a WebVTT timestamp string:
 * `HH:MM:SS.mmm`
 */
function toVttTimestamp(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  const ms = Math.round((seconds - Math.floor(seconds)) * 1000);
  return `${pad2(h)}:${pad2(m)}:${pad2(s)}.${pad3(ms)}`;
}

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function pad3(n: number): string {
  return String(n).padStart(3, "0");
}

// ---------------------------------------------------------------------------
// Word-highlight helpers (SRT / VTT)
// ---------------------------------------------------------------------------

/**
 * Build an SRT cue with word-level `<mark>` tags for the active word.
 * Each token gets its own numbered cue, re-showing the full sentence text
 * with one word highlighted at a time (matching parakeet-mlx behaviour).
 */
function srtWordHighlightCues(
  sentence: AlignedSentence,
  baseIndex: number
): string {
  return sentence.tokens
    .map((token, i) => {
      const highlighted = sentence.tokens
        .map((t, j) => (j === i ? `<u>${t.text}</u>` : t.text))
        .join("");
      return `${baseIndex + i}\n${toSrtTimestamp(token.start)} --> ${toSrtTimestamp(token.end)}\n${highlighted}\n`;
    })
    .join("\n");
}

/**
 * Build a VTT cue with word-level `<c>` tags for the active word (WebVTT
 * karaoke style, matching parakeet-mlx highlight behaviour).
 */
function vttWordHighlightCue(sentence: AlignedSentence): string {
  const tagged = sentence.tokens
    .map(
      (t: AlignedToken) =>
        `<${toVttTimestamp(t.start)}><c>${t.text}</c>`
    )
    .join(" ");
  return `${toVttTimestamp(sentence.start)} --> ${toVttTimestamp(sentence.end)}\n${tagged}\n`;
}

// ---------------------------------------------------------------------------
// Plain text
// ---------------------------------------------------------------------------

export function formatTxt(result: TranscriptionResult): string {
  return result.text.trim() + "\n";
}

// ---------------------------------------------------------------------------
// SRT
// ---------------------------------------------------------------------------

export function formatSrt(
  result: TranscriptionResult,
  highlightWords: boolean
): string {
  const parts: string[] = [];
  let cueIndex = 1;

  for (const sentence of result.sentences) {
    if (highlightWords && sentence.tokens.length > 0) {
      parts.push(srtWordHighlightCues(sentence, cueIndex));
      cueIndex += sentence.tokens.length;
    } else {
      parts.push(
        `${cueIndex}\n${toSrtTimestamp(sentence.start)} --> ${toSrtTimestamp(sentence.end)}\n${sentence.text.trim()}\n`
      );
      cueIndex++;
    }
  }

  return parts.join("\n");
}

// ---------------------------------------------------------------------------
// WebVTT
// ---------------------------------------------------------------------------

export function formatVtt(
  result: TranscriptionResult,
  highlightWords: boolean
): string {
  const header = "WEBVTT\n\n";
  const cues = result.sentences
    .map((sentence) => {
      if (highlightWords && sentence.tokens.length > 0) {
        return vttWordHighlightCue(sentence);
      }
      return `${toVttTimestamp(sentence.start)} --> ${toVttTimestamp(sentence.end)}\n${sentence.text.trim()}\n`;
    })
    .join("\n");

  return header + cues;
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

export function formatJson(result: TranscriptionResult): string {
  return JSON.stringify(result, null, 2) + "\n";
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/**
 * Return a map of format → formatted string for the requested output format.
 * When format is `"all"`, all four formats are included.
 */
export function formatAll(
  result: TranscriptionResult,
  format: OutputFormat,
  highlightWords: boolean
): Map<OutputFormat, string> {
  const map = new Map<OutputFormat, string>();

  const formats: Array<Exclude<OutputFormat, "all">> =
    format === "all" ? ["txt", "srt", "vtt", "json"] : [format as Exclude<OutputFormat, "all">];

  for (const fmt of formats) {
    switch (fmt) {
      case "txt":
        map.set("txt", formatTxt(result));
        break;
      case "srt":
        map.set("srt", formatSrt(result, highlightWords));
        break;
      case "vtt":
        map.set("vtt", formatVtt(result, highlightWords));
        break;
      case "json":
        map.set("json", formatJson(result));
        break;
    }
  }

  return map;
}
