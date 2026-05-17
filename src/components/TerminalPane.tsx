import React, { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { Unicode11Addon } from '@xterm/addon-unicode11';
import '@xterm/xterm/css/xterm.css';
import type { Project, Language, TerminalTab, TerminalExitEvent } from '../types';
import { listen } from '@tauri-apps/api/event';
import { getProjectColor } from '../lib/projectColors';
import { spawnTerminal, writeTerminal, resizeTerminal, closeTerminal } from '../lib/commands';
import { translate } from '../lib/i18n';
import { normalizeError } from '../lib/utils';

const waitForStableLayout = () =>
  new Promise<void>((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });

type TerminalOutputPayload = number[] | { data: number[] };

interface TerminalPaneProps {
  activeProject: Project;
  terminalTabs: TerminalTab[];
  activeTerminalId: string | null;
  onNewTerminal: () => void;
  onCloseTerminal: (id: string) => void;
  onSelectTerminal: (id: string) => void;
  isActive: boolean;
  lang: Language;
}

// ── Inline rename input component ─────────────────────────────────────
interface RenameInputProps {
  value: string;
  onSubmit: (name: string) => void;
  onCancel: () => void;
}

const RenameInput: React.FC<RenameInputProps> = ({ value, onSubmit, onCancel }) => {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    // Focus + select all text
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      const trimmed = inputRef.current?.value.trim() || '';
      if (trimmed.length > 0) {
        onSubmit(trimmed.slice(0, 64));
      } else {
        onCancel();
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onCancel();
    }
  };

  const handleBlur = () => {
    const trimmed = inputRef.current?.value.trim() || '';
    if (trimmed.length > 0) {
      onSubmit(trimmed.slice(0, 64));
    } else {
      onCancel();
    }
  };

  return (
    <input
      ref={inputRef}
      className="terminal-rename-input"
      defaultValue={value}
      onKeyDown={handleKeyDown}
      onBlur={handleBlur}
      maxLength={64}
      onClick={(e) => e.stopPropagation()}
    />
  );
};

// ── TerminalPane component ────────────────────────────────────────────
const TerminalPane: React.FC<TerminalPaneProps> = ({
  activeProject,
  terminalTabs,
  activeTerminalId,
  onNewTerminal,
  onCloseTerminal,
  onSelectTerminal,
  isActive,
  lang,
}) => {
  const terminalRefs = useRef<Record<string, HTMLDivElement | null>>({});
  const terminalInstances = useRef<Record<string, { term: Terminal; fitAddon: FitAddon }>>({});
  const realIdsRef = useRef<Record<string, string>>({});
  const cleanupFnsRef = useRef<Record<string, () => void>>({});
  const lastDimsRef = useRef<Record<string, { cols: number; rows: number }>>({});
  const resizeFrameRef = useRef<Record<string, number>>({});
  const decoderRef = useRef<Record<string, TextDecoder>>({});
  const [terminalNames, setTerminalNames] = useState<Record<string, string>>({});
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [pendingTerminalAction, setPendingTerminalAction] = useState<{ tempId: string; x: number; y: number } | null>(null);
  const [terminalContextMenu, setTerminalContextMenu] = useState<{ tempId: string; x: number; y: number } | null>(null);
  const [recentlyStopped, setRecentlyStopped] = useState<Record<string, boolean>>({});
  const terminalActionMenuRef = useRef<HTMLDivElement>(null);
  const terminalContextMenuRef = useRef<HTMLDivElement>(null);

  const t = useCallback((key: string) => translate(key, lang), [lang]);

  // ── Generate default name ───────────────────────────────────────────
  const getDefaultName = useCallback((index: number) => {
    return `Terminal ${index + 1}`;
  }, []);

  const getTerminalSize = useCallback((term: Terminal) => {
    const cols = Number.isFinite(term.cols) && term.cols > 0 ? term.cols : 80;
    const rows = Number.isFinite(term.rows) && term.rows > 0 ? term.rows : 24;
    return { cols, rows };
  }, []);

  const isElementVisible = useCallback((el: HTMLElement | null) => {
    return Boolean(el && el.clientWidth > 0 && el.clientHeight > 0);
  }, []);

  const isTerminalVisible = useCallback((tempId: string) => {
    return isElementVisible(terminalRefs.current[tempId]);
  }, [isElementVisible]);

  const fitTerminal = useCallback((tempId: string) => {
    const instance = terminalInstances.current[tempId];
    const container = terminalRefs.current[tempId];

    if (!instance || !isElementVisible(container)) {
      return { cols: 80, rows: 24 };
    }

    try {
      instance.fitAddon.fit();
    } catch (err) {
      console.warn('terminal fit ignored:', normalizeError(err));
    }

    return getTerminalSize(instance.term);
  }, [getTerminalSize, isElementVisible]);

  const fitAndResize = useCallback(async (tempId: string) => {
    const realId = realIdsRef.current[tempId];

    if (!realId || !isTerminalVisible(tempId)) return;

    const { cols, rows } = fitTerminal(tempId);
    const last = lastDimsRef.current[tempId];

    if (!last || last.cols !== cols || last.rows !== rows) {
      lastDimsRef.current[tempId] = { cols, rows };
      await resizeTerminal(realId, cols, rows);
    }
  }, [fitTerminal, isTerminalVisible]);

  const scheduleFitAndResize = useCallback((tempId: string) => {
    const previousFrame = resizeFrameRef.current[tempId];
    if (previousFrame) {
      cancelAnimationFrame(previousFrame);
    }

    resizeFrameRef.current[tempId] = requestAnimationFrame(() => {
      delete resizeFrameRef.current[tempId];
      fitAndResize(tempId);
    });
  }, [fitAndResize]);

  const closeMenus = useCallback(() => {
    setPendingTerminalAction(null);
    setTerminalContextMenu(null);
  }, []);

  const cleanupTerminalFrontend = useCallback((tempId: string) => {
    try {
      cleanupFnsRef.current[tempId]?.();
    } catch (_) {}
    try {
      terminalInstances.current[tempId]?.term.dispose();
    } catch (_) {}

    const resizeFrame = resizeFrameRef.current[tempId];
    if (resizeFrame) {
      cancelAnimationFrame(resizeFrame);
      delete resizeFrameRef.current[tempId];
    }
    delete lastDimsRef.current[tempId];
    delete decoderRef.current[tempId];
    delete cleanupFnsRef.current[tempId];
    delete terminalInstances.current[tempId];
    delete terminalRefs.current[tempId];
    delete realIdsRef.current[tempId];
    setTerminalNames(prev => {
      const next = { ...prev };
      delete next[tempId];
      return next;
    });
    setRecentlyStopped(prev => {
      const next = { ...prev };
      delete next[tempId];
      return next;
    });
    setRenamingId(prev => prev === tempId ? null : prev);
    closeMenus();
  }, [closeMenus]);

  const stopTerminalByTempId = useCallback(async (tempId: string) => {
    const realId = realIdsRef.current[tempId];
    const instance = terminalInstances.current[tempId];

    if (!realId) {
      instance?.term.write(`\r\n\x1b[33m[${translate('terminal.stop_signal_sent', lang)}]\x1b[0m\r\n`);
      return;
    }

    try {
      await writeTerminal(realId, '\x03');
      await new Promise(resolve => setTimeout(resolve, 120));
      await writeTerminal(realId, '\x03');
      instance?.term.write(`\r\n\x1b[33m[${translate('terminal.stop_signal_sent', lang)}]\x1b[0m\r\n`);
      setRecentlyStopped(prev => ({ ...prev, [tempId]: true }));
      window.setTimeout(() => {
        setRecentlyStopped(prev => {
          const next = { ...prev };
          delete next[tempId];
          return next;
        });
      }, 2500);
    } catch (err) {
      console.error('stopTerminal error:', normalizeError(err));
    }
  }, [lang]);

  const closeTerminalByTempId = useCallback(async (tempId: string) => {
    const realId = realIdsRef.current[tempId];
    if (realId) {
      try {
        await closeTerminal(realId);
      } catch (err) {
        console.warn('closeTerminal ignored:', normalizeError(err));
      }
    }
    cleanupTerminalFrontend(tempId);
    onCloseTerminal(tempId);
  }, [cleanupTerminalFrontend, onCloseTerminal]);

  const closeAllTerminals = useCallback(async () => {
    await Promise.all(terminalTabs.map(t => closeTerminalByTempId(t.id)));
  }, [terminalTabs, closeTerminalByTempId]);

  const closeOtherTerminals = useCallback(async (keepId: string) => {
    await Promise.all(terminalTabs.filter(t => t.id !== keepId).map(t => closeTerminalByTempId(t.id)));
    onSelectTerminal(keepId);
  }, [terminalTabs, closeTerminalByTempId, onSelectTerminal]);

  // ── Close terminal menus on click outside / Escape ──────────────────
  useEffect(() => {
    if (!terminalContextMenu && !pendingTerminalAction) return;

    const handlePointerDown = (e: PointerEvent) => {
      const target = e.target as Node;
      const insideContext = terminalContextMenuRef.current?.contains(target);
      const insideAction = terminalActionMenuRef.current?.contains(target);

      if (!insideContext && !insideAction) {
        closeMenus();
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        closeMenus();
      }
    };

    document.addEventListener('pointerdown', handlePointerDown, true);
    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.removeEventListener('pointerdown', handlePointerDown, true);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [terminalContextMenu, pendingTerminalAction, closeMenus]);

  // ── Spawn a terminal when a new temp ID appears ─────────────────────
  useEffect(() => {
    terminalTabs.forEach(async (tab, index) => {
      if (!terminalInstances.current[tab.id] && terminalRefs.current[tab.id]) {
        const id = tab.id;
        // Set default name if not set
        if (!terminalNames[tab.id]) {
          setTerminalNames(prev => ({ ...prev, [tab.id]: getDefaultName(index) }));
        }

        const term = new Terminal({
          cursorBlink: true,
          cursorStyle: 'block',
          fontSize: 14,
          fontFamily: "Consolas, 'Cascadia Mono', monospace",
          letterSpacing: 0,
          lineHeight: 1,
          theme: {
            background: '#1e1e2e',
            foreground: '#cdd6f4',
            cursor: '#f5e0dc',
            selectionBackground: '#585b70',
            black: '#45475a',
            red: '#f38ba8',
            green: '#a6e3a1',
            yellow: '#f9e2af',
            blue: '#89b4fa',
            magenta: '#f5c2e7',
            cyan: '#94e2d5',
            white: '#bac2de',
            brightBlack: '#585b70',
            brightRed: '#f38ba8',
            brightGreen: '#a6e3a1',
            brightYellow: '#f9e2af',
            brightBlue: '#89b4fa',
            brightMagenta: '#f5c2e7',
            brightCyan: '#94e2d5',
            brightWhite: '#a6adc8',
          },
          allowProposedApi: true,
        });

        term.attachCustomKeyEventHandler((ev) => {
          const key = ev.key?.toLowerCase();
          if (ev.type === 'keydown' && ev.ctrlKey && !ev.shiftKey && key === 'c') {
            ev.preventDefault();
            return false;
          }
          return true;
        });

        const unicode11Addon = new Unicode11Addon();
        term.loadAddon(unicode11Addon);
        term.unicode.activeVersion = '11';

        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);

        const container = terminalRefs.current[id];
        if (container) {
          term.open(container);
        }

        terminalInstances.current[id] = { term, fitAddon };

        // Spawn the actual terminal on the backend
        try {
          await waitForStableLayout();
          const initialSize = fitTerminal(id);

          const session = await spawnTerminal(
            tab.projectId,
            initialSize.cols,
            initialSize.rows,
            '',
            undefined,
          );

          const realId = session.id;
          realIdsRef.current[id] = realId;
          decoderRef.current[id] = new TextDecoder('utf-8', { fatal: false });
          const spawnSize = fitTerminal(id);
          lastDimsRef.current[id] = spawnSize;
          await resizeTerminal(realId, spawnSize.cols, spawnSize.rows);

          // Handle user input → send to backend
          const disposeInput = term.onData((data) => {
            if (data === '') {
              term.write(`\r\n\x1b[33m[${translate('terminal.ctrl_c_blocked', lang)}]\x1b[0m\r\n`);
              return;
            }
            writeTerminal(realId, data).catch((err) =>
              console.error('writeTerminal error:', normalizeError(err))
            );
          });

          // Listen for terminal output events from backend
          const unlistenOutput = await listen<TerminalOutputPayload>(
            `terminal://output/${realId}`,
            (event) => {
              const payload = Array.isArray(event.payload) ? event.payload : event.payload.data;
              let decoder = decoderRef.current[id];
              if (!decoder) {
                decoder = new TextDecoder('utf-8', { fatal: false });
                decoderRef.current[id] = decoder;
              }

              const text = decoder.decode(new Uint8Array(payload), { stream: true });
              if (text) {
                term.write(text);
              }
            }
          );

          // Listen for terminal exit events
          const unlistenExit = await listen<TerminalExitEvent>(
            `terminal://exit/${realId}`,
            (event) => {
              const payload = event.payload;
              const exitCode = payload.exitCode ?? 'unknown';
              const decoder = decoderRef.current[id];
              const remaining = decoder?.decode();
              if (remaining) {
                term.write(remaining);
              }
              term.write(`\r\n\x1b[2m[${translate('terminal.exit_prefix', lang)} ${exitCode}]\x1b[0m\r\n`);
              delete decoderRef.current[id];
              unlistenOutput();
              unlistenExit();
            }
          );

          // Fit terminal on container resize
          const resizeObserver = new ResizeObserver(() => {
            scheduleFitAndResize(id);
          });

          if (container) {
            resizeObserver.observe(container);
          }

          // Store cleanup function
          cleanupFnsRef.current[id] = () => {
            disposeInput.dispose();
            unlistenOutput();
            unlistenExit();
            resizeObserver.disconnect();
            delete decoderRef.current[id];
            const resizeFrame = resizeFrameRef.current[id];
            if (resizeFrame) {
              cancelAnimationFrame(resizeFrame);
              delete resizeFrameRef.current[id];
            }
          };
        } catch (err) {
          console.error('Failed to spawn terminal:', normalizeError(err));
          term.write(
            `\r\n\x1b[31m[${translate('terminal.spawn_error', lang)} - ${normalizeError(err)}]\x1b[0m\r\n`
          );
        }
      }
    });
    // terminalNames changes on rename but the guard `!terminalInstances.current[id]` prevents re-spawning
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [terminalTabs]);

  // ── Focus/refit the active terminal when it changes ────────────────
  useEffect(() => {
    if (isActive && activeTerminalId && terminalInstances.current[activeTerminalId]) {
      const instance = terminalInstances.current[activeTerminalId];
      if (instance) {
        requestAnimationFrame(() => {
          fitAndResize(activeTerminalId).finally(() => {
            instance.term.focus();
          });
        });
      }
    }
  }, [activeTerminalId, fitAndResize, isActive]);

  // ── Cleanup all terminals on unmount (NOT on tab switch) ───────────
  useEffect(() => {
    return () => {
      // Dispose all xterm instances
      Object.entries(terminalInstances.current).forEach(([_, { term }]) => {
        try {
          term.dispose();
        } catch (_) {}
      });
      // Run all cleanup functions
      Object.entries(cleanupFnsRef.current).forEach(([_, cleanup]) => {
        try {
          cleanup();
        } catch (_) {}
      });
      // Close all backend terminal sessions.
      Object.values(realIdsRef.current).forEach((realId) => {
        closeTerminal(realId).catch(() => {});
      });
    };
  }, []);

  // ── Rename handlers ─────────────────────────────────────────────────
  const handleStartRename = useCallback((id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setRenamingId(id);
  }, []);

  const handleFinishRename = useCallback((id: string, name: string) => {
    setTerminalNames(prev => ({ ...prev, [id]: name }));
    setRenamingId(null);
  }, []);

  const handleCancelRename = useCallback(() => {
    setRenamingId(null);
  }, []);

  // ── New Terminal handler ────────────────────────────────────────────
  const handleNewTerminal = useCallback(() => {
    onNewTerminal();
  }, [onNewTerminal]);

  // ── Render ──────────────────────────────────────────────────────────
  const activeTab = terminalTabs.find(t => t.id === activeTerminalId) ?? (terminalTabs.length > 0 ? terminalTabs[terminalTabs.length - 1] : undefined);
  const toolbarColor = activeTab?.projectColor ?? getProjectColor(activeProject);

  return (
    <div
      className="terminal-pane"
      style={{ '--project-color': toolbarColor } as React.CSSProperties}
    >
      <div className="terminal-toolbar">
        {terminalTabs.length > 0 && (
          <div className="terminal-tabs">
          {terminalTabs.map((tab, index) => {
            const id = tab.id;
            const isActive =
              activeTerminalId === id ||
              (activeTerminalId === null && index === terminalTabs.length - 1);
            return (
              <div
                key={id}
                className={`terminal-tab ${isActive ? 'active' : ''}`}
                style={{ '--terminal-project-color': tab.projectColor } as React.CSSProperties}
                onClick={() => onSelectTerminal(id)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  onSelectTerminal(id);
                  setTerminalContextMenu({ tempId: id, x: e.clientX, y: e.clientY });
                  setPendingTerminalAction(null);
                }}
              >
                <span className="terminal-project-dot" />
                {renamingId === id ? (
                  <RenameInput
                    value={terminalNames[id] || getDefaultName(index)}
                    onSubmit={(name) => handleFinishRename(id, name)}
                    onCancel={handleCancelRename}
                  />
                ) : (
                  <>
                    <span
                      className="terminal-tab-title"
                      onDoubleClick={(e) => handleStartRename(id, e)}
                      title={t('terminal.rename_hint')}
                    >
                      {terminalNames[id] || getDefaultName(index)}
                    </span>
                    {recentlyStopped[id] && (
                      <span className="terminal-status-badge">{t('terminal.stopped')}</span>
                    )}
                  </>
                )}
                <button
                  className="terminal-close-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    setPendingTerminalAction({ tempId: id, x: e.clientX, y: e.clientY });
                    setTerminalContextMenu(null);
                  }}
                  title={t('terminal.close')}
                >
                  &times;
                </button>
              </div>
            );
          })}
        </div>
        )}
        <div className="terminal-toolbar-actions">
          <button
            className="toolbar-btn toolbar-btn-new-terminal"
            onClick={handleNewTerminal}
            title={t('terminal.new_btn')}
          >
            {t('terminal.new_btn')}
          </button>
        </div>
      </div>
      {pendingTerminalAction && (
        <div
          className="terminal-action-menu"
          ref={terminalActionMenuRef}
          title={t('terminal.stop_or_close_title')}
          style={{ left: pendingTerminalAction.x, top: pendingTerminalAction.y }}
          onMouseDown={(e) => e.stopPropagation()}
        >
          <button onClick={() => { stopTerminalByTempId(pendingTerminalAction.tempId); closeMenus(); }}>
            {t('terminal.stop')}
          </button>
          <button onClick={() => { closeTerminalByTempId(pendingTerminalAction.tempId); closeMenus(); }}>
            {t('terminal.close')}
          </button>
        </div>
      )}
      {terminalContextMenu && (
        <div
          className="terminal-context-menu"
          ref={terminalContextMenuRef}
          style={{ left: terminalContextMenu.x, top: terminalContextMenu.y }}
          onMouseDown={(e) => e.stopPropagation()}
        >
          <div className="terminal-context-menu-header">
            <span>{t('terminal.context_menu_title')}</span>
            <button
              className="terminal-context-menu-close"
              onClick={() => setTerminalContextMenu(null)}
              title={t('terminal.context_menu_close')}
            >
              &times;
            </button>
          </div>
          <button className="terminal-context-menu-item" onClick={() => { const tempId = terminalContextMenu.tempId; setTerminalContextMenu(null); setRenamingId(tempId); }}>{t('terminal.context_rename')}</button>
          <button className="terminal-context-menu-item" onClick={() => { const tempId = terminalContextMenu.tempId; setTerminalContextMenu(null); stopTerminalByTempId(tempId); }}>{t('terminal.context_stop')}</button>
          <button className="terminal-context-menu-item" onClick={() => { const tempId = terminalContextMenu.tempId; setTerminalContextMenu(null); closeTerminalByTempId(tempId); }}>{t('terminal.context_close')}</button>
          <button className="terminal-context-menu-item" onClick={() => { setTerminalContextMenu(null); closeAllTerminals(); }}>{t('terminal.context_close_all')}</button>
          <button className="terminal-context-menu-item" onClick={() => { const tempId = terminalContextMenu.tempId; setTerminalContextMenu(null); closeOtherTerminals(tempId); }}>{t('terminal.context_close_others')}</button>
        </div>
      )}
      <div className="terminal-container">
        {terminalTabs.length === 0 ? (
          <div className="terminal-empty">
            <p>{t('terminal.no_terminal')}</p>
            <button className="toolbar-btn" onClick={handleNewTerminal}>
              {t('terminal.new_btn')}
            </button>
          </div>
        ) : (
          terminalTabs.map(tab => (
            <div
              key={tab.id}
              ref={(el) => {
                terminalRefs.current[tab.id] = el;
              }}
              className={`xterm-wrapper ${
                activeTerminalId === tab.id ||
                (activeTerminalId === null && tab.id === terminalTabs[terminalTabs.length - 1].id)
                  ? ''
                  : 'hidden'
              }`}
            />
          ))
        )}
      </div>
    </div>
  );
};

export default TerminalPane;