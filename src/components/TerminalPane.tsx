import React, { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import type { Project, Language } from '../types';
import { listen } from '@tauri-apps/api/event';
import { spawnTerminal, writeTerminal, resizeTerminal, killTerminal, isElevated, restartAsAdmin } from '../lib/commands';
import { translate } from '../lib/i18n';
import { normalizeError } from '../lib/utils';

interface TerminalPaneProps {
  activeProject: Project;
  terminalIds: string[];
  activeTerminalId: string | null;
  onNewTerminal: () => void;
  onCloseTerminal: (id: string) => void;
  onSelectTerminal: (id: string) => void;
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

// ── Admin confirm dialog component ────────────────────────────────────
interface AdminConfirmDialogProps {
  lang: Language;
  onRestartAsAdmin: () => void;
  onCancel: () => void;
}

const AdminConfirmDialog: React.FC<AdminConfirmDialogProps> = ({ lang, onRestartAsAdmin, onCancel }) => {
  const t = (key: string) => translate(key, lang);
  return (
    <div className="admin-confirm-overlay" onClick={onCancel}>
      <div className="admin-confirm-dialog" onClick={(e) => e.stopPropagation()}>
        <h3>{t('terminal.admin_confirm_title')}</h3>
        <p>{t('terminal.admin_confirm_text')}</p>
        <div className="admin-confirm-actions">
          <button className="admin-confirm-btn admin-confirm-btn-primary" onClick={onRestartAsAdmin}>
            {t('terminal.restart_as_admin')}
          </button>
          <button className="admin-confirm-btn admin-confirm-btn-cancel" onClick={onCancel}>
            {t('terminal.cancel')}
          </button>
        </div>
      </div>
    </div>
  );
};

// ── TerminalPane component ────────────────────────────────────────────
const TerminalPane: React.FC<TerminalPaneProps> = ({
  activeProject,
  terminalIds,
  activeTerminalId,
  onNewTerminal,
  onCloseTerminal,
  onSelectTerminal,
  lang,
}) => {
  const terminalRefs = useRef<Record<string, HTMLDivElement | null>>({});
  const terminalInstances = useRef<Record<string, { term: Terminal; fitAddon: FitAddon }>>({});
  const realIdsRef = useRef<Record<string, string>>({});
  const cleanupFnsRef = useRef<Record<string, () => void>>({});
  const elevatedRef = useRef<Record<string, boolean>>({});
  const [terminalNames, setTerminalNames] = useState<Record<string, string>>({});
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [showAdminDropdown, setShowAdminDropdown] = useState(false);
  const [adminConfirmOpen, setAdminConfirmOpen] = useState(false);
  const [adminChecking, setAdminChecking] = useState(false);
  const adminDropdownRef = useRef<HTMLDivElement>(null);

  const t = useCallback((key: string) => translate(key, lang), [lang]);

  // ── Generate default name ───────────────────────────────────────────
  const getDefaultName = useCallback((index: number, elevated: boolean) => {
    const base = `Terminal ${index + 1}`;
    return elevated ? `${base} (Admin)` : base;
  }, []);

  // ── Close admin dropdown on click outside ────────────────────────────
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (adminDropdownRef.current && !adminDropdownRef.current.contains(e.target as Node)) {
        setShowAdminDropdown(false);
      }
    };
    if (showAdminDropdown) {
      window.addEventListener('mousedown', handleClick);
    }
    return () => window.removeEventListener('mousedown', handleClick);
  }, [showAdminDropdown]);

  // ── Spawn a terminal when a new temp ID appears ─────────────────────
  useEffect(() => {
    terminalIds.forEach(async (id, index) => {
      if (!terminalInstances.current[id] && terminalRefs.current[id]) {
        // Set default name if not set
        if (!terminalNames[id]) {
          setTerminalNames(prev => ({ ...prev, [id]: getDefaultName(index, elevatedRef.current[id] || false) }));
        }

        const term = new Terminal({
          cursorBlink: true,
          cursorStyle: 'block',
          fontSize: 14,
          fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
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

        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);

        const container = terminalRefs.current[id];
        if (container) {
          term.open(container);
          // Fit after render
          requestAnimationFrame(() => {
            try {
              fitAddon.fit();
              term.focus();
            } catch (_) {}
          });
        }

        terminalInstances.current[id] = { term, fitAddon };

        // Spawn the actual terminal on the backend
        try {
          const dims = fitAddon.proposeDimensions();
          const cols = dims?.cols || 80;
          const rows = dims?.rows || 24;

          const isAdmin = elevatedRef.current[id] || false;

          const session = await spawnTerminal(
            activeProject.id,
            cols,
            rows,
            '',
            undefined,
            isAdmin,
          );

          const realId = session.id;
          realIdsRef.current[id] = realId;

          console.log(`[TerminalPane] Spawned terminal: tempId=${id}, realId=${realId}, elevated=${isAdmin}`);

          // Handle user input → send to backend
          const disposeInput = term.onData((data) => {
            writeTerminal(realId, data).catch((err) =>
              console.error('writeTerminal error:', normalizeError(err))
            );
          });

          // Listen for terminal output events from backend
          const unlistenOutput = await listen<string>(
            `terminal://output/${realId}`,
            (event) => {
              term.write(event.payload);
            }
          );

          // Listen for terminal exit events
          const unlistenExit = await listen(
            `terminal://exit/${realId}`,
            (event: any) => {
              const payload = event.payload;
              const exitCode = payload.exitCode ?? payload.exit_code ?? 'unknown';
              term.write(                    `\r\n\x1b[2m[${translate('terminal.exit_prefix', lang)} ${exitCode}]\x1b[0m\r\n`
              );
              unlistenOutput();
              unlistenExit();
            }
          );

          // Fit terminal on container resize
          const resizeObserver = new ResizeObserver(() => {
            requestAnimationFrame(() => {
              try {
                fitAddon.fit();
                const d = fitAddon.proposeDimensions();
                if (d) {
                  resizeTerminal(realId, d.cols, d.rows).catch(() => {});
                }
              } catch (_) {}
            });
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
  }, [terminalIds, activeProject.id]);

  // ── Focus/refit the active terminal when it changes ────────────────
  useEffect(() => {
    if (activeTerminalId && terminalInstances.current[activeTerminalId]) {
      const { term, fitAddon } = terminalInstances.current[activeTerminalId];
      requestAnimationFrame(() => {
        try {
          fitAddon.fit();
          term.focus();
        } catch (_) {}
      });
    }
  }, [activeTerminalId]);

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
      // Kill all backend terminals
      Object.values(realIdsRef.current).forEach((realId) => {
        killTerminal(realId).catch(() => {});
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
    elevatedRef.current[`new-${Date.now()}`] = false;
    onNewTerminal();
  }, [onNewTerminal]);

  // ── Admin Terminal handler ──────────────────────────────────────────
  const handleAdminTerminal = useCallback(async () => {
    setShowAdminDropdown(false);
    setAdminChecking(true);
    try {
      const elevated = await isElevated();
      if (elevated) {
        // App is already running as admin — spawn admin terminal directly
        elevatedRef.current[`new-${Date.now()}`] = true;
        onNewTerminal();
      } else {
        // App is not elevated — show dialog
        setAdminConfirmOpen(true);
      }
    } catch {
      // If isElevated fails (e.g., unsupported platform), show unavailable
      const isWindows = navigator.userAgent.includes('Windows');
      if (!isWindows) {
        alert(t('terminal.admin_unavailable'));
      } else {
        setAdminConfirmOpen(true);
      }
    } finally {
      setAdminChecking(false);
    }
  }, [onNewTerminal, t]);

  const handleRestartAsAdmin = useCallback(async () => {
    setAdminConfirmOpen(false);
    try {
      await restartAsAdmin();
    } catch (err) {
      // Restart not supported (Linux, dev mode, etc.) — show user-visible message
      alert(t('terminal.admin_unavailable'));
    }
  }, [t]);

  const handleCancelAdmin = useCallback(() => {
    setAdminConfirmOpen(false);
  }, []);

  // ── Render ──────────────────────────────────────────────────────────
  return (
    <div className="terminal-pane">
      <div className="terminal-toolbar">
        {terminalIds.length > 0 && (
          <div className="terminal-tabs">
          {terminalIds.map((id, index) => {
            const isActive =
              activeTerminalId === id ||
              (activeTerminalId === null && index === terminalIds.length - 1);
            return (
              <div
                key={id}
                className={`terminal-tab ${isActive ? 'active' : ''}`}
                onClick={() => onSelectTerminal(id)}
              >
                {renamingId === id ? (
                  <RenameInput
                    value={terminalNames[id] || getDefaultName(index, elevatedRef.current[id] || false)}
                    onSubmit={(name) => handleFinishRename(id, name)}
                    onCancel={handleCancelRename}
                  />
                ) : (
                  <span
                    onDoubleClick={(e) => handleStartRename(id, e)}
                    title="Double-click to rename"
                  >
                    {terminalNames[id] || getDefaultName(index, elevatedRef.current[id] || false)}
                    {elevatedRef.current[id] && (
                      <span className="terminal-admin-badge">{t('terminal.admin_badge')}</span>
                    )}
                  </span>
                )}
                <button
                  className="terminal-close-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    const realId = realIdsRef.current[id];
                    if (realId) {
                      killTerminal(realId).catch(console.error);
                    }
                    if (cleanupFnsRef.current[id]) {
                      cleanupFnsRef.current[id]();
                      delete cleanupFnsRef.current[id];
                    }
                    if (terminalInstances.current[id]) {
                      terminalInstances.current[id].term.dispose();
                      delete terminalInstances.current[id];
                    }
                    delete realIdsRef.current[id];
                    delete terminalRefs.current[id];
                    elevatedRef.current[id] = false;
                    onCloseTerminal(id);
                  }}
                >
                  &times;
                </button>
              </div>
            );
          })}
        </div>
        )}
        <div className="terminal-new-btn-group" ref={adminDropdownRef}>
          <button className="toolbar-btn toolbar-btn-new-terminal" onClick={handleNewTerminal} title={t('terminal.new_btn')}>
            {t('terminal.new_btn')}
          </button>
          <button
            className="toolbar-btn toolbar-btn-dropdown-toggle"
            onClick={() => setShowAdminDropdown(prev => !prev)}
            title={t('terminal.new_admin')}
            disabled={adminChecking}
          >
            ▾
          </button>
          {showAdminDropdown && (
            <div className="terminal-dropdown-menu">
              <button
                className="terminal-dropdown-item"
                onClick={handleAdminTerminal}
                disabled={adminChecking}
              >
                {t('terminal.new_admin')}
              </button>
            </div>
          )}
        </div>
      </div>
      {adminConfirmOpen && (
        <AdminConfirmDialog
          lang={lang}
          onRestartAsAdmin={handleRestartAsAdmin}
          onCancel={handleCancelAdmin}
        />
      )}
      <div className="terminal-container">
        {terminalIds.length === 0 ? (
          <div className="terminal-empty">
            <p>{t('terminal.no_terminal')}</p>
            <button className="toolbar-btn" onClick={handleNewTerminal}>
              {t('terminal.new_btn')}
            </button>
          </div>
        ) : (
          terminalIds.map((id) => (
            <div
              key={id}
              ref={(el) => {
                terminalRefs.current[id] = el;
              }}
              className={`xterm-wrapper ${
                activeTerminalId === id ||
                (activeTerminalId === null && id === terminalIds[terminalIds.length - 1])
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
