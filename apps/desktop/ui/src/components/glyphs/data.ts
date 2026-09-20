/* Lumi Glyph System data — ported verbatim from the vanilla app.js
   implementation (ICON.md §7: a port, not a redraw). Geometry, stroke
   widths, and optical sizes stay identical: 20×20 grid, 1.75 authored
   stroke rendered at 2px/16px, currentColor, angles 0/45/90. */

export const UTILITY_GLYPHS: Record<string, string> = {
  folder: '<path d="M2.75 5.25A1.5 1.5 0 0 1 4.25 3.75h3.4l1.9 2.5h6.2a1.5 1.5 0 0 1 1.5 1.5v7.5a1.5 1.5 0 0 1-1.5 1.5H4.25a1.5 1.5 0 0 1-1.5-1.5z"/>',
  folderOpen: '<path d="M2.75 5.25A1.5 1.5 0 0 1 4.25 3.75h3.4l1.9 2.5h6.2a1.5 1.5 0 0 1 1.5 1.5v1"/><path d="m2.75 14.75 1.6-4h12.4l-1.7 4z"/>',
  file: '<path d="M11.25 2.75H5.25A1.5 1.5 0 0 0 3.75 4.25v11.5a1.5 1.5 0 0 0 1.5 1.5h10a1.5 1.5 0 0 0 1.5-1.5V6.75z"/><path d="M11.25 2.75v4h4"/>',
  fileText: '<path d="M11.25 2.75H5.25A1.5 1.5 0 0 0 3.75 4.25v11.5a1.5 1.5 0 0 0 1.5 1.5h10a1.5 1.5 0 0 0 1.5-1.5V6.75z"/><path d="M11.25 2.75v4h4"/><path d="M6.75 10.25h6.5"/><path d="M6.75 13.25h4.5"/>',
  fileDiff: '<path d="M11.25 2.75H5.25A1.5 1.5 0 0 0 3.75 4.25v11.5a1.5 1.5 0 0 0 1.5 1.5h10a1.5 1.5 0 0 0 1.5-1.5V6.75z"/><path d="M11.25 2.75v4h4"/><path d="M10 10.25v4.5"/><path d="M7.75 12.5h4.5"/><path d="M7.75 15.5h4.5" stroke-width="1"/>',
  search: '<circle cx="8.75" cy="8.75" r="5.25"/><path d="m12.75 12.75 4.25 4.25"/>',
  plus: '<path d="M10 4.25v11.5"/><path d="M4.25 10h11.5"/>',
  close: '<path d="m5 5 10 10"/><path d="M15 5 5 15"/>',
  chevronRight: '<path d="M7.5 4.5 13 10l-5.5 5.5"/>',
  chevronDown: '<path d="M4.5 7.5 10 13l5.5-5.5"/>',
  chevronLeft: '<path d="M15.5 4.5 10 10l5.5 5.5"/>',
  copy: '<rect x="7.25" y="7.25" width="9" height="9" rx="1.5"/><path d="M12.75 4.75h-6a2 2 0 0 0-2 2v6"/>',
  refresh: '<path d="M16.75 10a6.75 6.75 0 0 0-13-2.5"/><path d="M3.75 3.25v3.5h3.5"/><path d="M3.25 10a6.75 6.75 0 0 0 13 2.5"/><path d="M16.25 16.75v-3.5h-3.5"/>',
  clock: '<circle cx="10" cy="10" r="6.75"/><path d="M10 6.25V10l2.5 1.75"/>',
  arrowRight: '<path d="M3.25 10h12.5"/><path d="m10.5 4.75 5.25 5.25-5.25 5.25"/>',
  external: '<path d="M11.25 3.25h5.5v5.5"/><path d="M15.75 4.25 8.25 11.75"/><path d="M16.75 11.75v3.5a1.5 1.5 0 0 1-1.5 1.5h-9.5a1.5 1.5 0 0 1-1.5-1.5v-9.5a1.5 1.5 0 0 1 1.5-1.5h3.5"/>',
  settings: '<path d="M17.25 4.5h-6"/><path d="M7.25 4.5H2.75"/><path d="M17.25 10h-4"/><path d="M9 10H2.75"/><path d="M17.25 15.5h-2.5"/><path d="M10.5 15.5H2.75"/><path d="M13.25 2.5v4"/><path d="M11 8v4"/><path d="M14.75 13.5v4"/>',
  list: '<path d="M6.75 4.5h10.5"/><path d="M6.75 10h10.5"/><path d="M6.75 15.5h10.5"/><path d="M3 4.5h.01"/><path d="M3 10h.01"/><path d="M3 15.5h.01"/>',
  edit: '<path d="m13.25 3 3.75 3.75L7.25 16.5 3 17.5l1-4.25z"/>',
  trash: '<path d="M3 4.75h14"/><path d="M6.75 4.75V3.25h6.5v1.5"/><path d="m5.25 4.75.7 11.25a1.5 1.5 0 0 0 1.5 1.4h5.1a1.5 1.5 0 0 0 1.5-1.4l.7-11.25"/><path d="M8.25 8v6"/><path d="M11.75 8v6"/>',
  eye: '<path d="M2 10s3-5.75 8-5.75 8 5.75 8 5.75-3 5.75-8 5.75S2 10 2 10z"/><circle cx="10" cy="10" r="2.5"/>',
  download: '<path d="M17.25 13.5v2.75a1.5 1.5 0 0 1-1.5 1.5h-11.5a1.5 1.5 0 0 1-1.5-1.5v-2.75"/><path d="m6 9.25 4 4 4-4"/><path d="M10 13.25V3"/>',
  inbox: '<path d="M17.5 10.5H13l-1.5 2.25h-3L7 10.5H2.5"/><path d="M4.75 3.75h10.5l2.25 6.75v4.75a1.5 1.5 0 0 1-1.5 1.5H4a1.5 1.5 0 0 1-1.5-1.5v-4.75z"/>',
  play: '<path d="m5.5 3.5 11 6.5-11 6.5z"/>',
  stop: '<rect x="4.75" y="4.75" width="10.5" height="10.5" rx="1.5"/>',
  lock: '<rect x="3.75" y="8.75" width="12.5" height="8" rx="1.5"/><path d="M6.75 8.75V6a3.25 3.25 0 0 1 6.5 0v2.75"/>',
  branch: '<circle cx="14.75" cy="5" r="2.25"/><circle cx="5.25" cy="15" r="2.25"/><path d="M5.25 12.75V6a3.5 3.5 0 0 1 3.5-3.5h1.5"/><path d="M12.25 6.75a6 6 0 0 1-4.9 5.9"/>',
  commit: '<circle cx="10" cy="10" r="2.75"/><path d="M2.75 10h4.5"/><path d="M12.75 10h4.5"/>',
  home: '<path d="M3 8.25 10 2.75l7 5.5v7.5a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 3 15.75z"/><path d="M7.5 17.5v-5h5v5"/>',
  history: '<path d="M3.25 10a6.75 6.75 0 1 0 2-4.75L2.75 7.75"/><path d="M2.75 3.25v4.5h4.5"/><path d="M10 6.75V10l2.5 1.5"/>',
  hash: '<path d="M3.5 7.5h13"/><path d="M3.5 12.5h13"/><path d="M8.5 3.5 7 16.5"/><path d="M13 3.5l-1.5 13"/>',
  layers: '<path d="M10 2.75 3 6.25l7 3.5 7-3.5z"/><path d="m3 10 7 3.5 7-3.5"/><path d="m3 13.75 7 3.5 7-3.5"/>',
  message: '<path d="M17.25 12.5a1.5 1.5 0 0 1-1.5 1.5H5.75l-3.5 3.5v-11.5a1.5 1.5 0 0 1 1.5-1.5h12a1.5 1.5 0 0 1 1.5 1.5z"/>',
  terminal: '<rect x="2.75" y="3.75" width="14.5" height="12.5" rx="1.5"/><path d="m5.75 7.25 2.25 2.25-2.25 2.25"/><path d="M10.25 12h4"/>',
  verified: '<circle cx="10" cy="10" r="6.75"/><path d="m7.25 10.25 1.9 1.9 3.85-4.4"/>',
  insight: '<path d="M10 2.75a5.5 5.5 0 0 0-3.25 9.94c.75.56 1.25 1.31 1.25 2.31h4c0-1 .5-1.75 1.25-2.31A5.5 5.5 0 0 0 10 2.75z"/><path d="M8 17.75h4"/><path d="M8.75 20h2.5"/>',
  activity: '<path d="M2.75 10h3.5l2.25-6.25 3.5 12 2.25-5.75h3.5"/>',

};

export const PRODUCT_GLYPHS: Record<string, string> = {
  project: '<path d="M3.25 17V7.25"/><path d="M3.25 17h10.5"/><rect x="8.25" y="4.75" width="8.5" height="8" rx="1"/>',
  task: '<path d="m3.25 10.25 3.25 3.25 7-7.25"/><circle cx="16.25" cy="14" r="1.75" fill="currentColor" stroke="none"/>',
  agent: '<path d="M3.5 16.5V8"/><path d="M3.5 16.5H11"/><circle cx="13.75" cy="6.25" r="2.25"/><path d="m8.5 12.5 3.4-3.9"/>',
  evidence: '<path d="M11.25 2.75H5.25A1.5 1.5 0 0 0 3.75 4.25v11.5a1.5 1.5 0 0 0 1.5 1.5h10a1.5 1.5 0 0 0 1.5-1.5V6.75z"/><path d="M11.25 2.75v4h4"/><circle cx="13.75" cy="14" r="1.5" fill="currentColor" stroke="none"/>',
  approval: '<circle cx="10" cy="10" r="6.75"/><path d="m7.25 10.25 1.9 1.9 3.85-4.4"/>',
  artifact: '<path d="M17.25 6.5 10 2.75 2.75 6.5v7L10 17.25l7.25-3.75z"/><path d="m2.75 6.5 7.25 3.75 7.25-3.75"/><path d="M10 10.25v7"/>',
  verification: '<circle cx="8.75" cy="8.75" r="5.25"/><path d="m12.5 12.5 4.25 4.25"/><path d="m6.75 8.75 1.5 1.5 2.75-3"/>',
  recovery: '<path d="M16.5 10a6.5 6.5 0 1 1-1.9-4.6"/><path d="M16.5 2.75v3.5H13"/><circle cx="10" cy="10" r="1.4" fill="currentColor" stroke="none"/>',
  exception: '<path d="M10 3 2.5 16.25h15z"/><path d="M10 8v3.25"/><circle cx="10" cy="13.75" r="1" fill="currentColor" stroke="none"/>',
  authority: '<path d="M10 2.5 16.5 5v5.25c0 4.1-2.9 6.75-6.5 8-3.6-1.25-6.5-3.9-6.5-8V5z"/><path d="M7.25 12V8.75h5.25"/>',
  guardrail: '<path d="M10 2.5 16.5 5v5.25c0 4.1-2.9 6.75-6.5 8-3.6-1.25-6.5-3.9-6.5-8V5z"/><path d="M7.25 10h5.25"/>',
  workflow: '<circle cx="4.75" cy="4.75" r="2"/><circle cx="15.25" cy="4.75" r="2"/><circle cx="10" cy="15.25" r="2"/><path d="M6.5 6.25 8.9 13.5"/><path d="M13.5 6.25 11.1 13.5"/><path d="M6.75 4.75h6.5"/>',

};

export const MARKS: Record<string, string> = {
  clarity: '<path d="M10 2.25 11.8 8.2 17.75 10 11.8 11.8 10 17.75 8.2 11.8 2.25 10l5.95-1.8z"/>',
  annotationDot: '<circle cx="10" cy="10" r="3.5" fill="currentColor" stroke="none"/>',
  lCorner: '<path d="M3.75 3.75v12.5h12.5"/>',
  diagonal: '<path d="M3.25 16.75 16.75 3.25"/>',

};

export const GLYPHS: Record<string, string> = {
  ...UTILITY_GLYPHS,
  ...PRODUCT_GLYPHS,
};
