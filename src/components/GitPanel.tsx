import React, { useState, useCallback, useMemo } from 'react';
import type {
  Project,
  GitStatus,
  GitFileStatus,
  GitDiffResult,
  Language,
  GitLoadState,
} from '../types';
import { getGitDiff } from '../lib/commands';
import { translate } from '../lib/i18n';
import { normalizeError } from '../lib/utils';

interface GitPanelProps {
  activeProject: Project;
  onOpenFile: (relativePath: string) => void;
  lang: Language;
  gitState: GitLoadState;
  gitStatus: GitStatus | null;
  gitError: string | null;
  skippedUntracked: boolean;
  onLoadGit: () => void;
  onRefreshGit: () => void;
}

const MAX_RENDER_FILES = 200;

const statusIcons: Record<string, string> = {
  modified: '\u{1F4DD}',
  added: '\u{2705}',
  deleted: '\u{1F5D1}\u{FE0F}',
  renamed: '\u{1F4CB}',
  untracked: '\u{2753}',
  conflicted: '\u{26A0}\u{FE0F}',
  unknown: '\u{2754}',
};

const statusColors: Record<string, string> = {
  modified: '#f9e2af',
  added: '#a6e3a1',
  deleted: '#f38ba8',
  renamed: '#89b4fa',
  untracked: '#fab387',
  conflicted: '#f38ba8',
  unknown: '#a6adc8',
};

const GitPanel: React.FC<GitPanelProps> = ({
  activeProject,
  onOpenFile,
  lang,
  gitState,
  gitStatus,
  gitError,
  skippedUntracked,
  onLoadGit,
  onRefreshGit,
}) => {
  const [diffContent, setDiffContent] = useState<GitDiffResult | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);

  const t = useCallback((key: string) => translate(key, lang), [lang]);

  const handleViewDiff = async (file: GitFileStatus) => {
    setDiffLoading(true);
    setDiffContent(null);
    try {
      const diff = await getGitDiff(activeProject.id, file.path);
      setDiffContent(diff);
    } catch (err: unknown) {
      console.error('Failed to load diff:', normalizeError(err));
    } finally {
      setDiffLoading(false);
    }
  };

  const groupFilesByStatus = (files: GitFileStatus[]): Record<string, GitFileStatus[]> => {
    const groups: Record<string, GitFileStatus[]> = {};
    for (const file of files) {
      const status = file.display_status;
      if (!groups[status]) {
        groups[status] = [];
      }
      groups[status].push(file);
    }
    return groups;
  };

  const groupedFiles = useMemo(() => {
    if (!gitStatus?.files) return {};
    const files = gitStatus.files.length > MAX_RENDER_FILES
      ? gitStatus.files.slice(0, MAX_RENDER_FILES)
      : gitStatus.files;
    return groupFilesByStatus(files);
  }, [gitStatus?.files]);

  const totalChanges = gitStatus?.files.length ?? 0;
  const truncated = totalChanges > MAX_RENDER_FILES;

  // ---- Idle: user hasn't clicked Load Git yet ----
  if (gitState === 'idle') {
    return (
      <div className="git-panel">
        <div className="git-idle">
          <div className="git-idle-icon">{'\u{2387}'}</div>
          <h3 className="git-idle-title">{t('git.idle_title')}</h3>
          <p className="git-idle-desc">{t('git.idle_desc')}</p>
          <button className="git-load-btn" onClick={onLoadGit}>
            {t('git.load')}
          </button>
        </div>
      </div>
    );
  }

  // ---- Loading ----
  if (gitState === 'loading') {
    return (
      <div className="git-panel">
        <div className="git-header">
          <h3>{t('git.title')}</h3>
        </div>
        <div className="git-loading">{t('git.loading')}</div>
      </div>
    );
  }

  // ---- Error ----
  if (gitState === 'error') {
    const isTimeout = gitError?.toLowerCase().includes('timeout') || gitError?.toLowerCase().includes('too long');
    return (
      <div className="git-panel">
        <div className="git-header">
          <h3>{t('git.title')}</h3>
          <button className="toolbar-btn" onClick={onRefreshGit}>{t('git.retry')}</button>
        </div>
        <div className="git-error">
          {isTimeout ? t('git.timeout') : `${t('git.unable')}: ${gitError}`}
        </div>
      </div>
    );
  }

  // ---- Loaded ----
  if (!gitStatus) {
    // Should not happen in "loaded" state, but handle gracefully
    return (
      <div className="git-panel">
        <div className="git-header">
          <h3>{t('git.title')}</h3>
          <button className="toolbar-btn" onClick={onRefreshGit}>{t('git.refresh')}</button>
        </div>
        <div className="git-empty">{t('git.unable')}</div>
      </div>
    );
  }

  if (!gitStatus.is_repo) {
    return (
      <div className="git-panel">
        <div className="git-header">
          <h3>{t('git.title')}</h3>
          <button className="toolbar-btn" onClick={onRefreshGit}>{t('git.reload')}</button>
        </div>
        <div className="git-not-repo">
          <p>{t('git.not_repo')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="git-panel">
      <div className="git-header">
        <div className="git-branch-info">
          <span className="git-branch-icon">{'\u{2387}'}</span>
          <span className="git-branch-name">{gitStatus.branch || 'unknown'}</span>
          {(gitStatus.ahead || gitStatus.behind) && (
            <span className="git-branch-stats">
              {gitStatus.ahead ? `\u{2191}${gitStatus.ahead}` : ''}
              {gitStatus.behind ? `\u{2193}${gitStatus.behind}` : ''}
            </span>
          )}
        </div>
        <button className="toolbar-btn" onClick={onRefreshGit} title={t('git.refresh')}>{t('git.refresh')}</button>
      </div>

      {skippedUntracked && (
        <div className="git-skipped-banner">{t('git.skipped_untracked')}</div>
      )}

      {truncated && (
        <div className="git-skipped-banner">{t('git.too_many_files')}</div>
      )}

      <div className="git-status-summary">
        {totalChanges > 0 ? (
          <span>{totalChanges} {t('git.changed_files')}</span>
        ) : (
          <span className="git-clean">{t('git.working_clean')}</span>
        )}
      </div>

      {totalChanges > 0 && (
        <div className="git-file-list">
          {Object.entries(groupedFiles).map(([status, files]) => (
            <div key={status} className="git-status-group">
              <div className="git-group-header">
                <span className="git-group-icon">{statusIcons[status] || '\u{2754}'}</span>
                <span style={{ color: statusColors[status] || '#cdd6f4' }}>
                  {status.charAt(0).toUpperCase() + status.slice(1)}
                </span>
                <span className="git-group-count">({files.length})</span>
              </div>
              {files.map(file => (
                <div key={file.path} className="git-file-item">
                  <span className="git-file-status-badge" style={{ color: statusColors[file.display_status] || '#cdd6f4' }}>
                    {file.index_status}{file.working_tree_status !== ' ' ? file.working_tree_status : ' '}
                  </span>
                  <span
                    className="git-file-path"
                    onClick={() => onOpenFile(file.path)}
                    title={t('git.open_file')}
                  >
                    {file.path}
                  </span>
                  <button
                    className="git-diff-btn"
                    onClick={() => handleViewDiff(file)}
                    title={t('git.diff')}
                  >
                    {t('git.diff')}
                  </button>
                </div>
              ))}
            </div>
          ))}
        </div>
      )}

      {diffLoading && (
        <div className="git-diff-loading">{t('git.loading_diff')}</div>
      )}

      {diffContent && (
        <div className="git-diff-viewer">
          <div className="git-diff-header">
            <span>Diff: {diffContent.path}</span>
            <button className="toolbar-btn-small" onClick={() => setDiffContent(null)}>
              &times;
            </button>
          </div>
          <pre className="git-diff-content">{diffContent.diff || t('git.no_diff')}</pre>
        </div>
      )}
    </div>
  );
};

export default GitPanel;