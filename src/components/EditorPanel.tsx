import React, { useEffect, useRef, useCallback, useState } from 'react';
import Editor, { loader } from '@monaco-editor/react';
import type { Project, OpenEditorTab, Language } from '../types';
import { readFile, writeFile } from '../lib/commands';
import { normalizeError } from '../lib/utils';
import { translate } from '../lib/i18n';

interface EditorPanelProps {
  activeProject: Project;
  editorTabs: OpenEditorTab[];
  activeEditorPath: string | null;
  onCloseEditor: (relativePath: string) => void;
  onCloseAllEditors: () => void;
  onCloseOtherEditors: (relativePath: string) => void;
  onSelectEditor: (relativePath: string) => void;
  onContentLoaded: (relativePath: string, content: string, languageId?: string) => void;
  onDirtyChange: (relativePath: string, dirty: boolean) => void;
  lang: Language;
}

// Configure Monaco Editor
loader.config({
  paths: {
    vs: 'https://cdn.jsdelivr.net/npm/monaco-editor@0.52.2/min/vs',
  },
});

const EditorPanel: React.FC<EditorPanelProps> = ({
  activeProject,
  editorTabs,
  activeEditorPath,
  onCloseEditor,
  onCloseAllEditors,
  onCloseOtherEditors,
  onSelectEditor,
  onContentLoaded,
  onDirtyChange,
  lang,
}) => {
  const editorRefs = useRef<Record<string, any>>({});
  const loadedContentRef = useRef<Record<string, boolean>>({});
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; relativePath: string } | null>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);

  const activeTab = editorTabs.find(t => t.relativePath === activeEditorPath);

  const t = (key: string) => translate(key, lang);

  // Load file content when a tab becomes active
  useEffect(() => {
    if (activeTab && activeTab.loading && !loadedContentRef.current[activeTab.relativePath]) {
      loadedContentRef.current[activeTab.relativePath] = true;
      (async () => {
        try {
          const result = await readFile(activeProject.path, activeTab.relativePath);
          onContentLoaded(activeTab.relativePath, result.content, result.language_id || undefined);
        } catch (err) {
          const msg = normalizeError(err);
          console.error('Failed to read file:', msg);
          onContentLoaded(activeTab.relativePath, `// Error loading file: ${msg}`);
        }
      })();
    }
  }, [activeTab, activeProject.path, onContentLoaded]);

  // Close context menu on Escape
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && contextMenu) {
        setContextMenu(null);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [contextMenu]);

  // Close context menu on click outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (contextMenu && contextMenuRef.current && !contextMenuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
    };
    window.addEventListener('mousedown', handleClickOutside);
    return () => window.removeEventListener('mousedown', handleClickOutside);
  }, [contextMenu]);

  // Close context menu when tabs change (activeEditorPath change, etc.)
  useEffect(() => {
    setContextMenu(null);
  }, [activeEditorPath, editorTabs.length]);

  const handleTabRightClick = useCallback((e: React.MouseEvent, relativePath: string) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, relativePath });
  }, []);

  const handleCloseAll = useCallback(() => {
    setContextMenu(null);
    loadedContentRef.current = {};
    editorRefs.current = {};
    onCloseAllEditors();
  }, [onCloseAllEditors]);

  const handleCloseOthers = useCallback(() => {
    if (contextMenu) {
      setContextMenu(null);
      const keepPath = contextMenu.relativePath;
      // Clear refs for all tabs except the kept one
      Object.keys(loadedContentRef.current).forEach(path => {
        if (path !== keepPath) {
          delete loadedContentRef.current[path];
          delete editorRefs.current[path];
        }
      });
      onCloseOtherEditors(keepPath);
    }
  }, [contextMenu, onCloseOtherEditors]);

  const handleEditorDidMount = useCallback((editor: any, monaco: any, relativePath: string) => {
    editorRefs.current[relativePath] = editor;

    // Save on Ctrl+S
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, async () => {
      const tab = editorTabs.find(t => t.relativePath === relativePath);
      if (!tab || !tab.dirty) return;

      const content = editor.getValue();
      try {
        await writeFile(activeProject.path, relativePath, content);
        onDirtyChange(relativePath, false);
        editorRefs.current[relativePath]?.getModel()?.setModified(false);        } catch (err) {
          console.error('Failed to save file:', normalizeError(err));
        }
    });
  }, [editorTabs, activeProject.path, onDirtyChange]);

  const handleEditorChange = useCallback((value: string | undefined, relativePath: string, originalContent: string | undefined) => {
    if (value !== undefined && originalContent !== undefined) {
      const isDirty = value !== originalContent;
      onDirtyChange(relativePath, isDirty);
    }
  }, [onDirtyChange]);

  const handleSave = async (relativePath: string) => {
    const tab = editorTabs.find(t => t.relativePath === relativePath);
    if (!tab || !tab.dirty) return;

    const editor = editorRefs.current[relativePath];
    if (!editor) return;

    const content = editor.getValue();
    try {
      await writeFile(activeProject.path, relativePath, content);
      onDirtyChange(relativePath, false);
      editor.getModel()?.setModified(false);
    } catch (err) {
      console.error('Failed to save file:', normalizeError(err));
    }
  };

  // Close editor with unsaved confirmation
  const handleClose = (relativePath: string) => {
    const tab = editorTabs.find(t => t.relativePath === relativePath);
    if (tab?.dirty) {
      const confirmed = window.confirm(`"${tab.name}" has unsaved changes. Close anyway?`);
      if (!confirmed) return;
    }
    loadedContentRef.current[relativePath] = false;
    delete editorRefs.current[relativePath];
    onCloseEditor(relativePath);
  };

  if (editorTabs.length === 0) {
    return (
      <div className="editor-panel">
        <div className="editor-empty">
          <p>{t('editor.empty')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="editor-panel">
      <div className="editor-tabs">
        {editorTabs.map(tab => (
          <div
            key={tab.relativePath}
            className={`editor-tab ${activeEditorPath === tab.relativePath ? 'active' : ''}`}
            onClick={() => onSelectEditor(tab.relativePath)}
            onContextMenu={(e) => handleTabRightClick(e, tab.relativePath)}
          >
            <span className="editor-tab-name">
              {tab.dirty && <span className="dirty-indicator">● </span>}
              {tab.name}
            </span>
            <button
              className="editor-tab-close"
              onClick={(e) => {
                e.stopPropagation();
                handleClose(tab.relativePath);
              }}
            >
              &times;
            </button>
          </div>
        ))}
      </div>

      {/* Context Menu */}
      {contextMenu && (
        <div
          ref={contextMenuRef}
          className="editor-context-menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          <button
            className="editor-context-menu-item"
            onClick={handleCloseAll}
          >
            {t('editor.close_all')}
          </button>
          <button
            className="editor-context-menu-item"
            onClick={handleCloseOthers}
          >
            {t('editor.close_others')}
          </button>
        </div>
      )}

      <div className="editor-container">
        {activeTab && (
          <>
            {activeTab.loading ? (
              <div className="editor-loading">{t('editor.loading')}</div>
            ) : (
              <Editor
                key={activeTab.relativePath}
                defaultLanguage={activeTab.languageId || 'plaintext'}
                defaultValue={activeTab.content || ''}
                theme="vs-dark"
                options={{
                  fontSize: 14,
                  minimap: { enabled: false },
                  scrollBeyondLastLine: false,
                  wordWrap: 'on',
                  automaticLayout: true,
                  readOnly: activeTab.content === undefined,
                }}
                onMount={(editor, monaco) => handleEditorDidMount(editor, monaco, activeTab.relativePath)}
                onChange={(value) => handleEditorChange(value, activeTab.relativePath, activeTab.content)}
              />
            )}
            <div className="editor-statusbar">
              <span className="status-item">
                {activeTab.relativePath}
              </span>
              <span className="status-item">
                {activeTab.languageId || 'Plain Text'}
              </span>
              <span className="status-item">
                {activeTab.dirty ? t('editor.unsaved') : t('editor.saved')}
              </span>
              {activeTab.dirty && (
                <button
                  className="toolbar-btn-small"
                  onClick={() => handleSave(activeTab.relativePath)}
                >
                  {t('editor.save')}
                </button>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
};

export default EditorPanel;
