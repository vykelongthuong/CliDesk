import React, { useState, useCallback, useRef } from 'react';
import TerminalPane from './TerminalPane';
import FileExplorer from './FileExplorer';
import EditorPanel from './EditorPanel';
import GitPanel from './GitPanel';
import SettingsPanel from './SettingsPanel';
import type { Project, OpenEditorTab, Language } from '../types';
import { translate } from '../lib/i18n';

type TabId = 'terminals' | 'files' | 'git' | 'settings';

interface WorkspaceProps {
  activeProject: Project | null;
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
  lang: Language;
  onLanguageChange: (lang: Language) => void;
}

const Workspace: React.FC<WorkspaceProps> = ({ activeProject, activeTab, onTabChange, lang, onLanguageChange }) => {
  const [terminalIds, setTerminalIds] = useState<string[]>([]);
  const [activeTerminalId, setActiveTerminalId] = useState<string | null>(null);
  const [editorTabs, setEditorTabs] = useState<OpenEditorTab[]>([]);
  const [activeEditorPath, setActiveEditorPath] = useState<string | null>(null);
  const terminalCountRef = useRef(0);

  const handleNewTerminal = useCallback(() => {
    terminalCountRef.current += 1;
    const tempId = `new-${terminalCountRef.current}`;
    setTerminalIds(prev => [...prev, tempId]);
    setActiveTerminalId(tempId);
    
    // After workspace renders, the terminal will spawn via backend
  }, []);

  const handleCloseTerminal = useCallback((id: string) => {
    setTerminalIds(prev => prev.filter(tid => tid !== id));
    setActiveTerminalId(prev => prev === id ? null : prev);
  }, []);

  const handleOpenFile = useCallback(async (relativePath: string) => {
    // Check if already open
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

  // Tabs configuration — Settings is now in sidebar, not here
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
            {tab.id === 'terminals' && terminalIds.length > 0 && (
              <span className="tab-badge">{terminalIds.length}</span>
            )}
          </button>
        ))}
      </div>

      <div className="workspace-content">
        {/* Always render all panels — use CSS display:none to hide inactive ones.
            This keeps xterm instances & backend terminals alive when switching tabs. */}
        <div className={`workspace-panel ${activeTab === 'terminals' ? 'active' : ''}`}>
          <TerminalPane
            activeProject={activeProject}
            terminalIds={terminalIds}
            activeTerminalId={activeTerminalId}
            onNewTerminal={handleNewTerminal}
            onCloseTerminal={handleCloseTerminal}
            onSelectTerminal={setActiveTerminalId}
            lang={lang}
          />
        </div>

        <div className={`workspace-panel ${activeTab === 'files' ? 'active' : ''}`}>
          <div className="files-workspace">
            <FileExplorer
              activeProject={activeProject}
              onOpenFile={handleOpenFile}
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
          <GitPanel activeProject={activeProject} onOpenFile={handleOpenFile} lang={lang} />
        </div>

        <div className={`workspace-panel ${activeTab === 'settings' ? 'active' : ''}`}>
          <SettingsPanel lang={lang} onLanguageChange={onLanguageChange} />
        </div>
      </div>
    </main>
  );
};

export default Workspace;
