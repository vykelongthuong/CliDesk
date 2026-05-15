import React, { useState } from 'react';
import type { Project, Language } from '../types';
import { addProject, removeProject } from '../lib/commands';
import { open } from '@tauri-apps/plugin-dialog';
import { translate } from '../lib/i18n';

interface SidebarProps {
  projects: Project[];
  activeProjectId: string | null;
  onSelectProject: (id: string) => void;
  onProjectsChanged: () => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
  onSettingsClick: () => void;
  lang: Language;
}

const Sidebar: React.FC<SidebarProps> = ({
  projects,
  activeProjectId,
  onSelectProject,
  onProjectsChanged,
  collapsed,
  onToggleCollapse,
  onSettingsClick,
  lang,
}) => {
  const [isAdding, setIsAdding] = useState(false);

  const t = (key: string) => translate(key, lang);

  const handleAddProject = async () => {
    try {
      setIsAdding(true);
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: 'Select Project Folder',
      });
      if (selectedPath) {
        const project = await addProject(selectedPath);
        onSelectProject(project.id);
        onProjectsChanged();
      }
    } catch (err) {
      console.error('Failed to add project:', err);
    } finally {
      setIsAdding(false);
    }
  };

  const handleRemoveProject = async (e: React.MouseEvent, projectId: string) => {
    e.stopPropagation();
    try {
      await removeProject(projectId);
      if (activeProjectId === projectId) {
        onSelectProject('');
      }
      onProjectsChanged();
    } catch (err) {
      console.error('Failed to remove project:', err);
    }
  };

  return (
    <aside className={`sidebar${collapsed ? ' collapsed' : ''}`}>
      <div className="sidebar-header">
        <div className="sidebar-title">
          <button
            className="sidebar-toggle-btn"
            onClick={onToggleCollapse}
            title={collapsed ? t('sidebar.expand') : t('sidebar.collapse')}
          >
            {collapsed ? '▶' : '◀'}
          </button>
          {!collapsed && <span>{t('app.title')}</span>}
        </div>
      </div>

      {!collapsed && (
        <div className="sidebar-actions">
          <button
            className="sidebar-add-btn"
            onClick={handleAddProject}
            disabled={isAdding}
          >
            {isAdding ? t('sidebar.adding') : t('sidebar.add_project')}
          </button>
        </div>
      )}

      <nav className="sidebar-nav">
        {projects.length === 0 ? (
          !collapsed && (
            <div className="sidebar-empty">
              <p>{t('sidebar.no_projects')}</p>
              <p className="sidebar-empty-hint">{t('sidebar.no_projects_hint')}</p>
            </div>
          )
        ) : (
          <ul className="project-list">
            {projects.map((project) => (
              <li
                key={project.id}
                className={`project-item ${activeProjectId === project.id ? 'active' : ''}`}
                onClick={() => onSelectProject(project.id)}
                title={collapsed ? project.name : undefined}
              >
                <div className="project-icon">
                  {project.name.charAt(0).toUpperCase()}
                </div>
                {!collapsed && (
                  <>
                    <div className="project-info">
                      <span className="project-name">{project.name}</span>
                      <span className="project-path">{project.path}</span>
                    </div>
                    <button
                      className="project-remove-btn"
                      onClick={(e) => handleRemoveProject(e, project.id)}
                      title="Remove from dashboard"
                    >
                      &times;
                    </button>
                  </>
                )}
              </li>
            ))}
          </ul>
        )}
      </nav>

      {/* Settings gear button */}
      <button
        className="sidebar-settings-btn"
        onClick={onSettingsClick}
        title={t('sidebar.settings')}
      >
        <span className="sidebar-settings-icon">⚙</span>
        {!collapsed && <span className="sidebar-settings-label">{t('sidebar.settings')}</span>}
      </button>
    </aside>
  );
};

export default Sidebar;
