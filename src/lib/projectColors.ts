import type { Project } from '../types';

/**
 * Palette of 12 colors designed for dark theme (Catppuccin-inspired).
 * Colors are chosen so that text (either light or dark) stays readable
 * on top of them when used as icon backgrounds or accent borders.
 */
const PALETTE: string[] = [
  '#89b4fa', // blue
  '#a6e3a1', // green
  '#f9e2af', // yellow
  '#f38ba8', // red
  '#cba6f7', // mauve
  '#94e2d5', // teal
  '#fab387', // peach
  '#b4befe', // lavender
  '#74c7ec', // sky
  '#f5c2e7', // pink
  '#eba0ac', // maroon
  '#a6adc8', // subtext
];

/** Simple djb2-style string hash — deterministic, no crypto dependency. */
function hashString(str: string): number {
  let hash = 5381;
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) + hash + str.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

/**
 * Returns a stable, per-project accent color.
 *
 * The seed is `project.id` when available, falling back to `project.path`
 * then `project.name`.  The same project always produces the same colour
 * across restarts.
 *
 * Returns `var(--accent)`-compatible fallback when `project` is nullish.
 */
export function getProjectColor(project: Project | null | undefined): string {
  if (!project) return '#89b4fa'; // fallback accent

  const seed = project.id || project.path || project.name || '';
  const index = hashString(seed) % PALETTE.length;
  return PALETTE[index];
}
