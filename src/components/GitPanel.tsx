import React, { useState, useEffect, useCallback } from 'react';
import type { Project, GitStatus, GitFileStatus, GitDiffResult, Language } from '../types';
import { getGitStatus, getGitDiff } from '../lib/commands';
import { translate } from '../lib/i18n';

interface GitPanelProps {
  activeProject: Project;
  onOpenFile: (relativePath: string) => void;
  lang: Language;
}

const statusIcons: Record<string, string> = {
  modified: '📝',
  added: '✅',
  deleted: '🗑️',
  renamed: '📋',
  untracked: '❓',
  conflicted: '⚠️',
  unknown: '❔',
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

const GitPanel: React.FC<GitPanelProps> = ({ activeProject, onOpenFile, lang }) => {
  const [gitStatus, setGitStatus] = useState<GitStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [diffContent, setDiffContent] = useState<GitDiffResult | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const t = useCallback((key: string) => translate(key, lang), [lang]);

  const loadStatus = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const status = await getGitStatus(activeProject.id);
      setGitStatus(status);
    } catch (err: any) {
      setError(err?.message || t('git.unable'));
      setGitStatus(null);
    } finally {
      setLoading(false);
    }
  }, [activeProject.id]);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  const handleViewDiff = async (file: GitFileStatus) => {
    setDiffLoading(true);
    setDiffContent(null);
    try {
      const diff = await getGitDiff(activeProject.id, file.path);
      setDiffContent(diff);
    } catch (err: any) {
      console.error('Failed to load diff:', err);
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

  if (loading) {
    return (
      <div className="git-panel">
        <div className="git-loading">{t('git.loading')}</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="git-panel">
        <div className="git-header">
          <h3>{t('git.title')}</h3>
          <button className="toolbar-btn" onClick={loadStatus}>{t('git.refresh')}</button>
        </div>
        <div className="git-error">{error}</div>
      </div>
    );
  }

  if (!gitStatus) {
    return (
      <div className="git-panel">
        <div className="git-header">
          <h3>{t('git.title')}</h3>
          <button className="toolbar-btn" onClick={loadStatus}>{t('git.refresh')}</button>
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
        </div>
        <div className="git-not-repo">
          <p>{t('git.not_repo')}</p>
        </div>
      </div>
    );
  }

  const groupedFiles = groupFilesByStatus(gitStatus.files);
  const totalChanges = gitStatus.files.length;

  return (
    <div className="git-panel">
      <div className="git-header">
        <div className="git-branch-info">
          <span className="git-branch-icon">⎇</span>
          <span className="git-branch-name">{gitStatus.branch || 'unknown'}</span>
          {(gitStatus.ahead || gitStatus.behind) && (
            <span className="git-branch-stats">
              {gitStatus.ahead ? `↑${gitStatus.ahead}` : ''}
              {gitStatus.behind ? `↓${gitStatus.behind}` : ''}
            </span>
          )}
        </div>
        <button className="toolbar-btn" onClick={loadStatus} title={t('git.refresh')}>{t('git.refresh')}</button>
      </div>

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
                <span className="git-group-icon">{statusIcons[status] || '❔'}</span>
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
