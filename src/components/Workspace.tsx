import React, { useState, useCallback, useRef } from 'react';
import TerminalPane from './TerminalPane';
import FileExplorer from './FileExplorer';
import EditorPanel from './EditorPanel';
import GitPanel from './GitPanel';
import SettingsPanel from './SettingsPanel';
import type {
  Project,
  OpenEditorTab,
  Language,
  TerminalTab,
  GitCacheEntry,
  GitLoadState,
  AppRuntimeInfo,
} from '../types';
import { getProjectColor } from '../lib/projectColors';
import { translate } from '../lib/i18n';
import { getGitStatus } from '../lib/commands';
import { normalizeError } from '../lib/utils';

type TabId = 'terminals' | 'files' | 'git' | 'settings';

interface WorkspaceProps {
  activeProject: Project | null;
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
  lang: Language;
  onLanguageChange: (lang: Language) => void;
  runtimeInfo: AppRuntimeInfo | null;
  updateState: 'idle' | 'updating' | 'success' | 'error';
  onUpdateCliDesk: () => void;
}

const Workspace: React.FC<WorkspaceProps> = ({
  activeProject,
  activeTab,
  onTabChange,
  lang,
  onLanguageChange,
  runtimeInfo,
  updateState,
  onUpdateCliDesk,
}) => {
  const [terminalTabs, setTerminalTabs] = useState<TerminalTab[]>([]);
  const [activeTerminalId, setActiveTerminalId] = useState<string | null>(null);
  const [editorTabs, setEditorTabs] = useState<OpenEditorTab[]>([]);
  const [activeEditorPath, setActiveEditorPath] = useState<string | null>(null);
  const terminalCountRef = useRef(0);
  const [gitCache, setGitCache] = useState<Record<string, GitCacheEntry>>({});
  const loadingProjectRef = useRef<string | null>(null);

  // Load Git status for a project (only called manually by user action)
  const loadGitForProject = useCallback(async (projectId: string) => {
    // Prevent duplicate concurrent requests for the same project
    setGitCache(prev => {
      const entry = prev[projectId];
      if (entry?.state === 'loading') return prev;
      return {
        ...prev,
        [projectId]: { state: 'loading' as GitLoadState, status: null, error: null },
      };
    });

    loadingProjectRef.current = projectId;

    try {
      const status = await getGitStatus(projectId);
      if (loadingProjectRef.current !== projectId) return;
      setGitCache(prev => ({
        ...prev,
        [projectId]: {
          state: 'loaded' as GitLoadState,
          status,
          error: null,
          loadedAt: Date.now(),
          slowMode: status.slow_mode ?? false,
          skippedUntracked: status.skipped_untracked ?? false,
        },
      }));
    } catch (err: unknown) {
      if (loadingProjectRef.current !== projectId) return;
      const msg = normalizeError(err);
      setGitCache(prev => ({
        ...prev,
        [projectId]: {
          state: 'error' as GitLoadState,
          status: null,
          error: msg,
          loadedAt: Date.now(),
        },
      }));
    }
  }, []);

  // User clicks "Load Git" — initial load
  const handleLoadGit = useCallback(() => {
    if (activeProject) {
      loadGitForProject(activeProject.id);
    }
  }, [activeProject, loadGitForProject]);

  // User clicks "Refresh" — force reload (works for loaded, error, and even not-repo state)
  const handleRefreshGit = useCallback(() => {
    if (activeProject) {
      loadGitForProject(activeProject.id);
    }
  }, [activeProject, loadGitForProject]);

  // Derive the Git entry for the active project
  const activeGitEntry: GitCacheEntry = activeProject
    ? (gitCache[activeProject.id] ?? { state: 'idle', status: null, error: null })
    : { state: 'idle', status: null, error: null };

  const handleNewTerminal = useCallback(() => {
    if (!activeProject) return;

    terminalCountRef.current += 1;
    const tempId = `new-${terminalCountRef.current}`;
    const projectColor = getProjectColor(activeProject);

    setTerminalTabs(prev => [
      ...prev,
      {
        id: tempId,
        projectId: activeProject.id,
        projectName: activeProject.name,
        projectPath: activeProject.path,
        projectColor,
      },
    ]);
    setActiveTerminalId(tempId);
  }, [activeProject]);

  const handleCloseTerminal = useCallback((id: string) => {
    setTerminalTabs(prev => prev.filter(t => t.id !== id));
    setActiveTerminalId(prev => prev === id ? null : prev);
  }, []);

  const handleOpenFile = useCallback(async (relativePath: string) => {
    const existing = editorTabs.find(t => t.relativePath === relativePath);
    if (existing) {
      setActiveEditorPath(relativePath);
      return;
    }

    const name = relativePath.split('/').pop() || relativePath;
    setEditorTabs(prev => [
      ...prev,
      {
        relativePath,
        name,
        content: undefined,
        dirty: false,
        loading: true,
      },
    ]);
    setActiveEditorPath(relativePath);
    onTabChange('files');
  }, [editorTabs, onTabChange]);

  const handleCloseEditor = useCallback((relativePath: string) => {
    setEditorTabs(prev => prev.filter(t => t.relativePath !== relativePath));
    setActiveEditorPath(prev => prev === relativePath ? null : prev);
  }, []);

  const handleCloseAllEditors = useCallback(() => {
    setEditorTabs([]);
    setActiveEditorPath(null);
  }, []);

  const handleCloseOtherEditors = useCallback((relativePath: string) => {
    setEditorTabs(prev => prev.filter(t => t.relativePath === relativePath));
    setActiveEditorPath(relativePath);
  }, []);

  const handleEditorContentLoaded = useCallback((relativePath: string, content: string, languageId?: string) => {
    setEditorTabs(prev =>
      prev.map(t =>
        t.relativePath === relativePath
          ? { ...t, content, languageId, loading: false }
          : t
      )
    );
  }, []);

  const handleDirtyChange = useCallback((relativePath: string, dirty: boolean) => {
    setEditorTabs(prev =>
      prev.map(t =>
        t.relativePath === relativePath ? { ...t, dirty } : t
      )
    );
  }, []);

  const tabs: { id: TabId; labelKey: string }[] = [
    { id: 'terminals', labelKey: 'tab.terminals' },
    { id: 'files', labelKey: 'tab.files' },
    { id: 'git', labelKey: 'tab.git' },
  ];

  const t = (key: string) => translate(key, lang);

  if (!activeProject) {
    return (
      <main className="workspace">
        <div className="empty-state">
          <div className="empty-state-icon">&#128187;</div>
          <h2>{t('welcome.title')}</h2>
          <p>{t('welcome.desc')}</p>
          <div className="empty-state-steps">
            <div className="step">
              <span className="step-number">1</span>
              <span dangerouslySetInnerHTML={{ __html: t('welcome.step1') }} />
            </div>
            <div className="step">
              <span className="step-number">2</span>
              <span>{t('welcome.step2')}</span>
            </div>
            <div className="step">
              <span className="step-number">3</span>
              <span>{t('welcome.step3')}</span>
            </div>
          </div>
        </div>
      </main>
    );
  }

  return (
    <main className="workspace">
      <div className="workspace-tabs">
        {tabs.map(tab => (
          <button
            key={tab.id}
            className={`workspace-tab ${activeTab === tab.id ? 'active' : ''}`}
            onClick={() => onTabChange(tab.id)}
          >
            {t(tab.labelKey)}
            {tab.id === 'terminals' && terminalTabs.length > 0 && (
              <span className="tab-badge">{terminalTabs.length}</span>
            )}
          </button>
        ))}
      </div>

      <div className="workspace-content">
        <div className={`workspace-panel ${activeTab === 'terminals' ? 'active' : ''}`}>
          <TerminalPane
            activeProject={activeProject}
            terminalTabs={terminalTabs}
            activeTerminalId={activeTerminalId}
            onNewTerminal={handleNewTerminal}
            onCloseTerminal={handleCloseTerminal}
            onSelectTerminal={setActiveTerminalId}
            isActive={activeTab === 'terminals'}
            lang={lang}
          />
        </div>

        <div className={`workspace-panel ${activeTab === 'files' ? 'active' : ''}`}>
          <div className="files-workspace">
            <FileExplorer
              activeProject={activeProject}
              onOpenFile={handleOpenFile}
              isActive={activeTab === 'files'}
              lang={lang}
            />
            <EditorPanel
              activeProject={activeProject}
              editorTabs={editorTabs}
              activeEditorPath={activeEditorPath}
              onCloseEditor={handleCloseEditor}
              onCloseAllEditors={handleCloseAllEditors}
              onCloseOtherEditors={handleCloseOtherEditors}
              onSelectEditor={setActiveEditorPath}
              onContentLoaded={handleEditorContentLoaded}
              onDirtyChange={handleDirtyChange}
              lang={lang}
            />
          </div>
        </div>

        <div className={`workspace-panel ${activeTab === 'git' ? 'active' : ''}`}>
          <GitPanel
            activeProject={activeProject}
            onOpenFile={handleOpenFile}
            lang={lang}
            gitState={activeGitEntry.state}
            gitStatus={activeGitEntry.status}
            gitError={activeGitEntry.error}
            skippedUntracked={activeGitEntry.skippedUntracked ?? false}
            onLoadGit={handleLoadGit}
            onRefreshGit={handleRefreshGit}
          />
        </div>

        <div className={`workspace-panel ${activeTab === 'settings' ? 'active' : ''}`}>
          <SettingsPanel
            lang={lang}
            onLanguageChange={onLanguageChange}
            runtimeInfo={runtimeInfo}
            updateState={updateState}
            onUpdateCliDesk={onUpdateCliDesk}
          />
        </div>
      </div>
    </main>
  );
};

export default Workspace;
