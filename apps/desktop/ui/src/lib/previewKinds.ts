/* Pure preview-kind detection (Spec 30.3/30.9): the extension drives
   only the rendering choice; content sniffing (text vs binary) stays
   with the caller. No React, no IPC — unit-testable in isolation.
   Unsupported formats state so explicitly instead of faking a viewer:
   legacy Office binaries, macro-enabled files, and codecs the webview
   cannot decode stay honest-unsupported. */

export const IMAGE_MIME: Record<string, string> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  webp: "image/webp",
  gif: "image/gif",
  bmp: "image/bmp",
  ico: "image/x-icon",
  avif: "image/avif",
  svg: "image/svg+xml",
};

export const VIDEO_MIME: Record<string, string> = {
  mp4: "video/mp4",
  m4v: "video/mp4",
  webm: "video/webm",
  mov: "video/quicktime",
};

export const AUDIO_MIME: Record<string, string> = {
  mp3: "audio/mpeg",
  wav: "audio/wav",
  ogg: "audio/ogg",
  oga: "audio/ogg",
  m4a: "audio/mp4",
  aac: "audio/aac",
  flac: "audio/flac",
};

/** Zip containers without a dedicated viewer: a read-only entry
    listing (extraction is a gated write, not a preview action). */
export const ZIP_TYPES = new Set(["zip"]);

/** Genuinely unsupported by an honest label: legacy binary Office
    (doc/xls/ppt), macro-enabled files, and image codecs the webview
    cannot decode (heic/tiff). */
export const UNSUPPORTED_TYPES = new Set([
  "doc", "xls", "ppt",
  "docm", "xlsm", "pptm",
  "heic", "tiff", "tif",
]);

export type PreviewKind =
  | "text"
  | "markdown"
  | "image"
  | "media"
  | "csv"
  | "pdf"
  | "docx"
  | "xlsx"
  | "pptx"
  | "zip"
  | "unsupported";

export function extensionOf(path: string): string {
  const base = path.split("/").pop() ?? path;
  const dot = base.lastIndexOf(".");
  return dot === -1 ? "" : base.slice(dot + 1).toLowerCase();
}

/** Content sniffing overrides the filename (Spec 30.3: never trust the
    extension alone); the extension drives only the rendering choice. */
export function previewKindFor(path: string, isTextFile: boolean): PreviewKind {
  const ext = extensionOf(path);
  if (ext === "md" || ext === "markdown") return "markdown";
  if (ext in IMAGE_MIME) return "image";
  if (ext in VIDEO_MIME || ext in AUDIO_MIME) return "media";
  if (ext === "csv" || ext === "tsv") return "csv";
  if (ext === "pdf") return "pdf";
  if (ext === "docx") return "docx";
  if (ext === "xlsx") return "xlsx";
  if (ext === "pptx") return "pptx";
  if (ZIP_TYPES.has(ext)) return "zip";
  if (UNSUPPORTED_TYPES.has(ext)) return "unsupported";
  return isTextFile ? "text" : "unsupported";
}

/** The MIME type for data-URL rendering; unknown → octet-stream. */
export function mimeFor(path: string): string {
  const ext = extensionOf(path);
  return (
    IMAGE_MIME[ext] ??
    VIDEO_MIME[ext] ??
    AUDIO_MIME[ext] ??
    "application/octet-stream"
  );
}

/** Kinds whose viewers self-fetch bytes over the bounded base64 IPC —
    they must never go through the text read (it refuses binaries). */
export function isBinaryKind(kind: PreviewKind): boolean {
  return kind !== "text" && kind !== "markdown" && kind !== "csv";
}
