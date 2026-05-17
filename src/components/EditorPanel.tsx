import React, { useEffect, useRef, useCallback, useState } from 'react';
import Editor, { loader } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import type { Project, OpenEditorTab, Language } from '../types';
import { readFile, writeFile } from '../lib/commands';
import { normalizeError } from '../lib/utils';
import { translate } from '../lib/i18n';

// Use locally-bundled Monaco instead of CDN to avoid network latency in release builds.
loader.config({ monaco });

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
  theme: 'dark' | 'light';
}

type MarkdownViewMode = 'edit' | 'preview' | 'split';

const isMarkdownFile = (path: string): boolean => /\.(md|markdown)$/i.test(path);

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
  theme,
}) => {
  const editorRefs = useRef<Record<string, any>>({});
  const loadedContentRef = useRef<Record<string, boolean>>({});
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; relativePath: string } | null>(null);
  const [markdownViewMode, setMarkdownViewMode] = useState<MarkdownViewMode>('edit');
  const [editorValues, setEditorValues] = useState<Record<string, string>>({});
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
          setEditorValues(prev => ({ ...prev, [activeTab.relativePath]: result.content }));
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

  const handleCloseCurrent = useCallback(() => {
    if (activeEditorPath) {
      onCloseEditor(activeEditorPath);
    }
  }, [activeEditorPath, onCloseEditor]);

  const handleCloseAll = useCallback(() => {
    onCloseAllEditors();
    setContextMenu(null);
  }, [onCloseAllEditors]);

  const handleCloseOthers = useCallback(() => {
    if (activeEditorPath) {
      onCloseOtherEditors(activeEditorPath);
      setContextMenu(null);
    }
  }, [activeEditorPath, onCloseOtherEditors]);

  // Memoized handleEditorDidMount
  const handleEditorDidMount = useCallback((editor: any, _monaco: any, relativePath: string) => {
    editorRefs.current[relativePath] = editor;
  }, []);

  // Memoized handleEditorChange
  const handleEditorChange = useCallback((value: string | undefined, relativePath: string, originalContent: string | undefined) => {
    if (value !== undefined) {
      setEditorValues(prev => ({ ...prev, [relativePath]: value }));
      const dirty = value !== (originalContent ?? '');
      onDirtyChange(relativePath, dirty);
    }
  }, [onDirtyChange]);

  // Save handler
  const handleSave = useCallback(async (relativePath: string) => {
    const content = editorValues[relativePath];
    if (content !== undefined) {
      try {
        await writeFile(activeProject.path, relativePath, content);
        onContentLoaded(relativePath, content);
        onDirtyChange(relativePath, false);
      } catch (err) {
        console.error('Failed to save file:', normalizeError(err));
      }
    }
  }, [editorValues, activeProject.path, onContentLoaded, onDirtyChange]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === 's') {
        e.preventDefault();
        if (activeEditorPath) {
          handleSave(activeEditorPath);
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeEditorPath, handleSave]);

  return (
    <div className="editor-panel">
      {/* Editor Tabs */}
      <div className="editor-tabs">
        {editorTabs.map(tab => (
          <div
            key={tab.relativePath}
            className={`editor-tab ${activeEditorPath === tab.relativePath ? 'active' : ''} ${tab.dirty ? 'dirty' : ''}`}
            onClick={() => onSelectEditor(tab.relativePath)}
            onContextMenu={(e) => handleTabRightClick(e, tab.relativePath)}
          >
            <span className="editor-tab-name">
              {tab.dirty && <span className="editor-dirty-dot">●</span>}
              {tab.name}
            </span>
            <button
              className="editor-tab-close"
              onClick={(e) => {
                e.stopPropagation();
                onCloseEditor(tab.relativePath);
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
          className="editor-context-menu"
          ref={contextMenuRef}
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          <button
            className="editor-context-menu-item"
            onClick={handleCloseCurrent}
          >
            {t('editor.close')}
          </button>
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
            {isMarkdownFile(activeTab.relativePath) && !activeTab.loading && (
              <div className="markdown-toolbar">
                {(['edit', 'preview', 'split'] as MarkdownViewMode[]).map(mode => (
                  <button
                    key={mode}
                    className={`markdown-mode-btn ${markdownViewMode === mode ? 'active' : ''}`}
                    onClick={() => setMarkdownViewMode(mode)}
                  >
                    {mode === 'edit' && t('editor.markdown_edit')}
                    {mode === 'preview' && t('editor.markdown_preview')}
                    {mode === 'split' && t('editor.markdown_split')}
                  </button>
                ))}
              </div>
            )}
            {activeTab.loading ? (
              <div className="editor-loading">{t('editor.loading')}</div>
            ) : (() => {
              const isMarkdown = isMarkdownFile(activeTab.relativePath);
              const content = editorValues[activeTab.relativePath] ?? activeTab.content ?? '';
              const editor = (
                <Editor
                  key={activeTab.relativePath}
                  defaultLanguage={activeTab.languageId || 'plaintext'}
                  value={content}
                  theme={theme === 'dark' ? 'vs-dark' : 'vs'}
                  options={{
                    fontSize: 14,
                    minimap: { enabled: false },
                    scrollBeyondLastLine: false,
                    wordWrap: 'on',
                    automaticLayout: true,
                    readOnly: activeTab.content === undefined,
                  }}
                  onMount={(editor, monacoInstance) => handleEditorDidMount(editor, monacoInstance, activeTab.relativePath)}
                  onChange={(value) => handleEditorChange(value, activeTab.relativePath, activeTab.content)}
                />
              );
              const preview = (
                <div className="markdown-preview">
                  <div className="markdown-preview-content">
                    {content.trim() ? (
                      <ReactMarkdown remarkPlugins={[remarkGfm]}>{content}</ReactMarkdown>
                    ) : (
                      <p className="markdown-preview-empty">{t('editor.markdown_preview_empty')}</p>
                    )}
                  </div>
                </div>
              );

              if (!isMarkdown || markdownViewMode === 'edit') return editor;
              if (markdownViewMode === 'preview') return preview;
              return (
                <div className="markdown-split">
                  <div className="markdown-editor-half">{editor}</div>
                  <div className="markdown-preview-half">{preview}</div>
                </div>
              );
            })()}
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
