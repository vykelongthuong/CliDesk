/**
 * Normalize an unknown error value to a human-readable string.
 * Handles Error objects, Tauri invoke errors, plain objects, and strings.
 */
export function normalizeError(err: unknown): string {
  if (err instanceof Error) {
    return err.message;
  }
  if (typeof err === 'string') {
    return err;
  }
  if (err && typeof err === 'object') {
    const obj = err as Record<string, unknown>;
    // Tauri invoke errors often have a `message` field
    if (typeof obj.message === 'string') {
      return obj.message;
    }
    // Tauri errors may have a `name` or `code`
    if (typeof obj.code === 'string' && typeof obj.message === 'string') {
      return `[${obj.code}] ${obj.message}`;
    }
    if (typeof obj.code === 'string') {
      return obj.code;
    }
    // Safe JSON stringify (avoid circular reference crashes)
    try {
      return JSON.stringify(err);
    } catch {
      return String(err);
    }
  }
  return String(err);
}
