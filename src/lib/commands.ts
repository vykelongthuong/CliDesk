import { invoke } from '@tauri-apps/api/core';
import type {
  Project,
  TerminalSession,
  ShellConfig,
  FileTreeItem,
  FileReadResult,
  FileWriteResult,
  GitStatus,
  GitDiffResult,
} from '../types';

// Project commands
export async function listProjects(): Promise<Project[]> {
  return invoke('project_list');
}

export async function addProject(path: string): Promise<Project> {
  return invoke('project_add', { path });
}

export async function removeProject(projectId: string): Promise<void> {
  await invoke('project_remove', { projectId });
}

export async function selectProject(projectId: string): Promise<Project> {
  return invoke('project_select', { projectId });
}

// Terminal commands
export async function spawnTerminal(
  projectId: string,
  cols: number,
  rows: number,
  cwdRelativePath?: string,
  shellId?: string,
  elevated?: boolean
): Promise<TerminalSession> {
  return invoke('terminal_spawn', {
    projectId,
    cwdRelativePath: cwdRelativePath || '',
    shellId,
    cols,
    rows,
    elevated: elevated || false,
  });
}

export async function writeTerminal(terminalId: string, data: string): Promise<void> {
  await invoke('terminal_write', { terminalId, data });
}

export async function resizeTerminal(terminalId: string, cols: number, rows: number): Promise<void> {
  await invoke('terminal_resize', { terminalId, cols, rows });
}

export async function killTerminal(terminalId: string): Promise<void> {
  await invoke('terminal_kill', { terminalId });
}

export async function closeTerminal(terminalId: string): Promise<void> {
  await invoke('terminal_close', { terminalId });
}

export async function killAllTerminals(): Promise<{ killed: number }> {
  return invoke('terminal_kill_all');
}

export async function listTerminals(projectId: string): Promise<TerminalSession[]> {
  return invoke('terminal_list', { projectId });
}

export async function getShells(): Promise<ShellConfig[]> {
  return invoke('shell_list');
}

// File commands
export async function listDirectory(projectId: string, relativePath: string): Promise<FileTreeItem[]> {
  return invoke('fs_list_dir', { projectId, relativePath });
}

export async function readFile(projectId: string, relativePath: string): Promise<FileReadResult> {
  return invoke('fs_read_file', { projectId, relativePath });
}

export async function writeFile(projectId: string, relativePath: string, content: string): Promise<FileWriteResult> {
  return invoke('fs_write_file', { projectId, relativePath, content });
}

// Git commands
export async function getGitStatus(projectId: string): Promise<GitStatus> {
  return invoke('git_status', { projectId });
}

export async function getGitDiff(projectId: string, relativePath: string, staged?: boolean): Promise<GitDiffResult> {
  return invoke('git_diff', { projectId, relativePath, staged });
}

// Admin/Restart commands
export async function isElevated(): Promise<boolean> {
  return invoke('is_elevated');
}

export async function restartAsAdmin(): Promise<void> {
  await invoke('restart_as_admin');
}

// Settings commands
export async function getSettings(): Promise<Record<string, string>> {
  return invoke('settings_get');
}

export async function setSetting(key: string, value: string): Promise<void> {
  await invoke('settings_set', { key, value });
}

// Window/Tray commands
export async function hideWindow(): Promise<void> {
  await invoke('window_hide');
}

export async function showWindow(): Promise<void> {
  await invoke('window_show');
}

export async function quitApp(): Promise<void> {
  await invoke('quit_app');
}
