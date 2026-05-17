export type Language = 'vi' | 'en';

export interface Project {
  id: string;
  name: string;
  path: string;
  created_at: string;
  updated_at: string;
  last_opened_at: string | null;
}

/** Frontend-only tab descriptor — not persisted to the database. */
export interface TerminalTab {
  id: string;            // temporary frontend ID (e.g. "new-1")
  projectId: string;
  projectName: string;
  projectPath: string;
  projectColor: string;
}

export interface TerminalSession {
  id: string;
  project_id: string;
  title: string;
  cwd: string;
  shell: ShellConfig;
  status: 'starting' | 'running' | 'exited' | 'killed' | 'error';
  exit_code: number | null;
  created_at: string;
}

export interface ShellConfig {
  id: string;
  label: string;
  executable: string;
  args: string[];
}

export interface FileTreeItem {
  name: string;
  relative_path: string;
  kind: 'file' | 'directory' | 'symlink' | 'unknown';
  size: number | null;
  modified_at: string | null;
}

export interface FileReadResult {
  project_id: string;
  relative_path: string;
  content: string;
  language_id: string | null;
  size: number;
  modified_at: string;
  read_only: boolean;
}

export interface FileWriteResult {
  ok: boolean;
  modified_at: string;
}

export interface GitStatus {
  is_repo: boolean;
  branch: string | null;
  ahead: number | null;
  behind: number | null;
  files: GitFileStatus[];
  slow_mode?: boolean;
  skipped_untracked?: boolean;
}

export interface GitFileStatus {
  path: string;
  index_status: string;
  working_tree_status: string;
  display_status: string;
}

export interface GitDiffResult {
  path: string;
  diff: string;
}

export interface AppError {
  code: string;
  message: string;
}

export interface AppRuntimeInfo {
  current_version: string;
  latest_version: string | null;
  update_available: boolean;
  launch_language: Language | null;
  update_command: string;
}

export interface UpdateResult {
  ok: boolean;
  message: string;
}

export interface TerminalOutputEvent {
  terminalId: string;
  data: string;
}

export interface TerminalExitEvent {
  terminalId: string;
  exitCode?: number;
  reason: 'exited' | 'killed' | 'error';
  message?: string;
}

export type GitLoadState = 'idle' | 'loading' | 'loaded' | 'error';

export interface GitCacheEntry {
  state: GitLoadState;
  status: GitStatus | null;
  error: string | null;
  loadedAt?: number;
  slowMode?: boolean;
  skippedUntracked?: boolean;
}

export type OpenEditorTab = {
  relativePath: string;
  name: string;
  languageId?: string;
  content?: string;
  modifiedAt?: string;
  dirty: boolean;
  loading: boolean;
};
