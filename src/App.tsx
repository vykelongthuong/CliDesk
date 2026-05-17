import React, { useState, useCallback, useEffect, useRef } from 'react';
import Sidebar from './components/Sidebar';
import Workspace from './components/Workspace';
import type { Project, Language, AppRuntimeInfo } from './types';
import {
  listProjects,
  hideWindow,
  quitApp,
  getSettings,
  setSetting,
  getAppRuntimeInfo,
  updateCliDeskFromNpm,
} from './lib/commands';
import { listen } from '@tauri-apps/api/event';
import { translate } from './lib/i18n';

type TabId = 'terminals' | 'files' | 'git' | 'settings';

const App: React.FC = () => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [activeProjectId, setActiveProjectId] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<TabId>('terminals');
  const [closeDialogOpen, setCloseDialogOpen] = useState(false);
  const [runtimeInfo, setRuntimeInfo] = useState<AppRuntimeInfo | null>(null);
  const [updateState, setUpdateState] = useState<'idle' | 'updating' | 'success' | 'error'>('idle');
  const dialogShownRef = useRef(false);
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(() => {
    try {
      return localStorage.getItem('clidesk_sidebar_collapsed') === 'true';
    } catch {
      return false;
    }
  });
  const [lang, setLang] = useState<Language>('vi');

  const loadRuntimeInfo = useCallback(async () => {
    try {
      const info = await getAppRuntimeInfo();
      setRuntimeInfo(info);
      return info;
    } catch (err) {
      console.error('Failed to load runtime info:', err);
      return null;
    }
  }, []);

  // Load launch metadata and language from backend settings on mount
  useEffect(() => {
    (async () => {
      const info = await loadRuntimeInfo();
      try {
        const settings = await getSettings();
        const savedLang = settings['ui.language'];
        const launchLang = info?.launch_language;

        if (launchLang === 'en' || launchLang === 'vi') {
          setLang(launchLang);
          if (savedLang !== launchLang) {
            await setSetting('ui.language', launchLang);
          }
        } else if (savedLang === 'en' || savedLang === 'vi') {
          setLang(savedLang);
        }
      } catch (err) {
        console.error('Failed to load language setting:', err);
        // Default to Vietnamese
      }
    })();
  }, [loadRuntimeInfo]);

  const loadProjects = useCallback(async () => {
    try {
      const projects = await listProjects();
      setProjects(projects);
    } catch (err) {
      console.error('Failed to load projects:', err);
    }
  }, []);

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  // Listen for close-requested event from backend
  useEffect(() => {
    const setup = async () => {
      const unlisten = await listen('app://close-requested', () => {
        // Prevent showing multiple dialogs if user clicks X rapidly
        if (!dialogShownRef.current) {
          dialogShownRef.current = true;
          setCloseDialogOpen(true);
        }
      });
      return unlisten;
    };

    const unlistenPromise = setup();
    return () => {
      unlistenPromise.then(unlisten => unlisten());
    };
  }, []);

  const handleHideToTray = useCallback(async () => {
    dialogShownRef.current = false;
    setCloseDialogOpen(false);
    try {
      await hideWindow();
    } catch (err) {
      console.error('Failed to hide window:', err);
    }
  }, []);

  const handleQuitApp = useCallback(async () => {
    dialogShownRef.current = false;
    setCloseDialogOpen(false);
    try {
      await quitApp();
    } catch (err) {
      console.error('Failed to quit app:', err);
    }
  }, []);

  const handleCancelClose = useCallback(() => {
    dialogShownRef.current = false;
    setCloseDialogOpen(false);
  }, []);

  const handleToggleSidebar = useCallback(() => {
    setSidebarCollapsed(prev => {
      const next = !prev;
      try {
        localStorage.setItem('clidesk_sidebar_collapsed', String(next));
      } catch {}
      return next;
    });
  }, []);

  const handleSettingsClick = useCallback(() => {
    setActiveTab('settings');
  }, []);

  const handleLanguageChange = useCallback((newLang: Language) => {
    setLang(newLang);
  }, []);

  const handleUpdateCliDesk = useCallback(async () => {
    if (updateState === 'updating') return;

    setUpdateState('updating');
    try {
      await updateCliDeskFromNpm();
      setUpdateState('success');
      await loadRuntimeInfo();
    } catch (err) {
      console.error('Failed to update CliDesk:', err);
      setUpdateState('error');
    }
  }, [loadRuntimeInfo, updateState]);

  const activeProject = projects.find(p => p.id === activeProjectId) || null;

  const t = useCallback((key: string) => translate(key, lang), [lang]);

  return (
    <div className="app-layout">
      <Sidebar
        projects={projects}
        activeProjectId={activeProjectId}
        onSelectProject={(id) => setActiveProjectId(id)}
        onProjectsChanged={loadProjects}
        collapsed={sidebarCollapsed}
        onToggleCollapse={handleToggleSidebar}
        onSettingsClick={handleSettingsClick}
        lang={lang}
        runtimeInfo={runtimeInfo}
        updateState={updateState}
        onUpdateCliDesk={handleUpdateCliDesk}
      />
      <Workspace
        activeProject={activeProject}
        activeTab={activeTab}
        onTabChange={setActiveTab}
        lang={lang}
        onLanguageChange={handleLanguageChange}
        runtimeInfo={runtimeInfo}
        updateState={updateState}
        onUpdateCliDesk={handleUpdateCliDesk}
      />

      {closeDialogOpen && (
        <div className="close-modal-overlay" onClick={handleCancelClose}>
          <div className="close-modal" onClick={(e) => e.stopPropagation()}>
            <h3 className="close-modal-title">{t('close_modal.title')}</h3>
            <p className="close-modal-text">
              {t('close_modal.text')}
            </p>
            <div className="close-modal-actions">
              <button
                className="close-modal-btn close-modal-btn-secondary"
                onClick={handleHideToTray}
              >
                {t('close_modal.hide')}
              </button>
              <button
                className="close-modal-btn close-modal-btn-danger"
                onClick={handleQuitApp}
              >
                {t('close_modal.quit')}
              </button>
              <button
                className="close-modal-btn close-modal-btn-cancel"
                onClick={handleCancelClose}
              >
                {t('close_modal.cancel')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default App;

