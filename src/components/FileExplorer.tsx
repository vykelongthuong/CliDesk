import React, { useState, useEffect, useCallback } from 'react';
import type { Project, FileTreeItem, Language } from '../types';
import { listDirectory } from '../lib/commands';
import { translate } from '../lib/i18n';

interface FileExplorerProps {
  activeProject: Project;
  onOpenFile: (relativePath: string) => void;
  lang: Language;
}

interface TreeNode {
  item: FileTreeItem;
  children: TreeNode[] | null;
  expanded: boolean;
}

const FileExplorer: React.FC<FileExplorerProps> = ({ activeProject, onOpenFile, lang }) => {
  const [tree, setTree] = useState<Record<string, TreeNode>>({});
  const [loading, setLoading] = useState(true);

  const loadDirectory = useCallback(async (relativePath: string) => {
    try {
      const items = await listDirectory(activeProject.path, relativePath);
      return items;
    } catch (err) {
      console.error('Failed to load directory:', err);
      return [];
    }
  }, [activeProject.path]);

  const loadRoot = useCallback(async () => {
    setLoading(true);
    try {
      const items = await loadDirectory('');
      const rootNodes: Record<string, TreeNode> = {};
      for (const item of items) {
        rootNodes[item.relative_path] = {
          item,
          children: item.kind === 'directory' ? [] : null,
          expanded: false,
        };
      }
      setTree(rootNodes);
    } catch (err) {
      console.error('Failed to load root:', err);
    } finally {
      setLoading(false);
    }
  }, [loadDirectory]);

  useEffect(() => {
    loadRoot();
  }, [loadRoot]);

  const toggleExpand = useCallback(async (relativePath: string) => {
    const node = tree[relativePath];
    if (!node || node.item.kind !== 'directory') return;

    // If already expanded, collapse it
    if (node.expanded) {
      setTree(prev => ({
        ...prev,
        [relativePath]: { ...prev[relativePath], expanded: false },
      }));
      return;
    }

    // If children already loaded, just expand
    if (node.children && node.children.length > 0) {
      setTree(prev => ({
        ...prev,
        [relativePath]: { ...prev[relativePath], expanded: true },
      }));
      return;
    }

    // Load children from backend
    try {
      const items = await loadDirectory(relativePath);
      const newTree = { ...tree };
      newTree[relativePath] = { ...newTree[relativePath], expanded: true };

      // Add children to tree
      for (const item of items) {
        newTree[item.relative_path] = {
          item,
          children: item.kind === 'directory' ? [] : null,
          expanded: false,
        };
      }

      // Set children references
      newTree[relativePath] = {
        ...newTree[relativePath],
        children: items
          .filter(i => i.kind === 'directory')
          .map(i => newTree[i.relative_path]),
        expanded: true,
      };

      setTree(newTree);
    } catch (err) {
      console.error('Failed to expand directory:', err);
    }
  }, [tree, loadDirectory]);

  const renderTree = (items: [string, TreeNode][], depth: number = 0): React.ReactNode => {
    // Sort: directories first, then files, alphabetically
    const sorted = [...items].sort((a, b) => {
      const aIsDir = a[1].item.kind === 'directory';
      const bIsDir = b[1].item.kind === 'directory';
      if (aIsDir && !bIsDir) return -1;
      if (!aIsDir && bIsDir) return 1;
      return a[1].item.name.localeCompare(b[1].item.name);
    });

    return sorted.map(([path, node]) => (
      <div key={path}>
        <div
          className={`file-tree-item ${node.item.kind}`}
          style={{ paddingLeft: `${depth * 16 + 8}px` }}
          onClick={() => {
            if (node.item.kind === 'directory') {
              toggleExpand(path);
            } else {
              onOpenFile(path);
            }
          }}
        >
          <span className="file-tree-icon">
            {node.item.kind === 'directory' ? (node.expanded ? '▼' : '▶') : '📄'}
          </span>
          <span className="file-tree-name">{node.item.name}</span>
        </div>
        {node.item.kind === 'directory' && node.expanded && node.children !== null && (
          <div className="file-tree-children">
            {renderTree(
              Object.entries(tree).filter(([p]) => {
                // Only show direct children (path starts with this directory path + "/")
                const prefix = path + '/';
                return p.startsWith(prefix) && !p.slice(prefix.length).includes('/');
              }),
              depth + 1
            )}
          </div>
        )}
      </div>
    ));
  };

  return (
    <div className="file-explorer">
      <div className="file-explorer-header">
        <span className="file-explorer-title">
          {activeProject.name}
        </span>
        <button className="toolbar-btn-small" onClick={loadRoot} title="Refresh">
          ↻
        </button>
      </div>
      <div className="file-tree">
        {loading ? (
          <div className="file-tree-loading">{translate('files.loading', lang)}</div>
        ) : Object.keys(tree).length === 0 ? (
          <div className="file-tree-empty">{translate('files.no_files', lang)}</div>
        ) : (
          renderTree(Object.entries(tree))
        )}
      </div>
    </div>
  );
};

export default FileExplorer;
