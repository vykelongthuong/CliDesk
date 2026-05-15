import React, { useState, useEffect, useCallback } from 'react';
import type { Language } from '../types';
import { getSettings, setSetting, hideWindow } from '../lib/commands';
import { translate } from '../lib/i18n';

interface SettingsPanelProps {
  lang: Language;
  onLanguageChange: (lang: Language) => void;
}

const SettingsPanel: React.FC<SettingsPanelProps> = ({ lang, onLanguageChange }) => {
  const [settings, setSettings] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<string | null>(null);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  const t = useCallback((key: string) => translate(key, lang), [lang]);

  const loadSettings = useCallback(async () => {
    setLoading(true);
    try {
      const s = await getSettings();
      setSettings(s);
    } catch (err: any) {
      console.error('Failed to load settings:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  const handleChange = async (key: string, value: string) => {
    setSaving(key);
    setMessage(null);
    try {
      await setSetting(key, value);
      setSettings(prev => ({ ...prev, [key]: value }));
      setMessage({ type: 'success', text: t('settings.saved') });
    } catch (err: any) {
      setMessage({ type: 'error', text: err?.message || t('settings.save_failed') });
    } finally {
      setSaving(null);
    }
  };

  if (loading) {
    return (
      <div className="settings-scroll">
        <div className="settings-content">
          <div className="settings-loading">{t('settings.loading')}</div>
        </div>
      </div>
    );
  }

  return (
    <div className="settings-scroll">
      <div className="settings-content">
      <h2 className="settings-title">{translate('settings.title', lang)}</h2>

      {message && (
        <div className={`settings-message settings-message-${message.type}`}>
          {message.text}
        </div>
      )}

      <div className="settings-section">
        <h3 className="settings-section-title">{translate('settings.security', lang)}</h3>
        <div className="setting-item">
          <div className="setting-info">
            <label className="setting-label">{translate('settings.security_mode', lang)}</label>
            <p className="setting-description">{translate('settings.security_desc', lang)}</p>
          </div>            <select
            className="setting-select"
            value={settings['security.mode'] || 'standard'}
            onChange={(e) => handleChange('security.mode', e.target.value)}
            disabled={saving === 'security.mode'}
          >
            <option value="relaxed">{translate('settings.security_relaxed', lang)}</option>
            <option value="standard">{translate('settings.security_standard', lang)}</option>
            <option value="strict">{translate('settings.security_strict', lang)}</option>
          </select>
        </div>
      </div>

      <div className="settings-section">
        <h3 className="settings-section-title">{translate('settings.appearance', lang)}</h3>

        <div className="setting-item">
          <div className="setting-info">
            <label className="setting-label">{translate('settings.language', lang)}</label>
            <p className="setting-description">{translate('settings.language_desc', lang)}</p>
          </div>
          <select
            className="setting-select"
            value={lang}
            onChange={(e) => {
              const newLang = e.target.value as Language;
              handleChange('ui.language', newLang);
              onLanguageChange(newLang);
            }}
          >
            <option value="vi">{translate('settings.language_vi', lang)}</option>
            <option value="en">{translate('settings.language_en', lang)}</option>
          </select>
        </div>

        <div className="setting-item">
          <div className="setting-info">
            <label className="setting-label">{translate('settings.theme', lang)}</label>
            <p className="setting-description">{translate('settings.theme_desc', lang)}</p>
          </div>
          <select
            className="setting-select"
            value={settings['ui.theme'] || 'dark'}
            onChange={(e) => handleChange('ui.theme', e.target.value)}
            disabled={saving === 'ui.theme'}
          >
            <option value="dark">{translate('settings.theme_dark', lang)}</option>
            <option value="light">{translate('settings.theme_light', lang)}</option>
          </select>
        </div>
      </div>

      <div className="settings-section">
        <h3 className="settings-section-title">{translate('settings.terminal', lang)}</h3>
        <div className="setting-item">
          <div className="setting-info">
            <label className="setting-label">{translate('settings.terminal_font', lang)}</label>
            <p className="setting-description">{translate('settings.terminal_font_desc', lang)}</p>
          </div>
          <input
            type="number"
            className="setting-input"
            value={settings['terminal.fontSize'] || '14'}
            min={8}
            max={72}
            onChange={(e) => handleChange('terminal.fontSize', e.target.value)}
            disabled={saving === 'terminal.fontSize'}
          />
        </div>
      </div>

      <div className="settings-section">
        <h3 className="settings-section-title">{translate('settings.editor', lang)}</h3>
        <div className="setting-item">
          <div className="setting-info">
            <label className="setting-label">{translate('settings.editor_font', lang)}</label>
            <p className="setting-description">{translate('settings.editor_font_desc', lang)}</p>
          </div>
          <input
            type="number"
            className="setting-input"
            value={settings['editor.fontSize'] || '14'}
            min={8}
            max={72}
            onChange={(e) => handleChange('editor.fontSize', e.target.value)}
            disabled={saving === 'editor.fontSize'}
          />
        </div>
      </div>

      <div className="settings-section">
        <h3 className="settings-section-title">{translate('settings.tray', lang)}</h3>
        <div className="setting-item">
          <div className="setting-info">
            <label className="setting-label">{translate('settings.close_to_tray', lang)}</label>
            <p className="setting-description">{translate('settings.close_to_tray_desc', lang)}</p>
          </div>            <select
            className="setting-select"
            value={settings['tray.close_to_tray'] || 'false'}
            onChange={(e) => handleChange('tray.close_to_tray', e.target.value)}
            disabled={saving === 'tray.close_to_tray'}
          >
            <option value="false">{translate('settings.close_to_tray_off', lang)}</option>
            <option value="true">{translate('settings.close_to_tray_on', lang)}</option>
          </select>
        </div>
        <div className="setting-item">
          <div className="setting-info">
            <label className="setting-label">{translate('settings.minimize_to_tray', lang)}</label>
            <p className="setting-description">{translate('settings.minimize_to_tray_desc', lang)}</p>
          </div>            <select
            className="setting-select"
            value={settings['tray.minimize_to_tray'] || 'false'}
            onChange={(e) => handleChange('tray.minimize_to_tray', e.target.value)}
            disabled={saving === 'tray.minimize_to_tray'}
          >
            <option value="false">{translate('settings.minimize_to_tray_off', lang)}</option>
            <option value="true">{translate('settings.minimize_to_tray_on', lang)}</option>
          </select>
        </div>
        <div className="setting-item">
          <div className="setting-info">
            <label className="setting-label">{translate('settings.hide_now', lang)}</label>
            <p className="setting-description">{translate('settings.hide_now_desc', lang)}</p>
          </div>
          <button
            className="toolbar-btn"
            onClick={() => hideWindow().catch(console.error)}
          >
            {translate('settings.hide_now', lang)}
          </button>
        </div>
      </div>

      <div className="settings-section">
        <h3 className="settings-section-title">{translate('settings.about', lang)}</h3>
        <div className="setting-item">
          <div className="setting-info">
            <label className="setting-label">CliDesk</label>
            <p className="setting-description" dangerouslySetInnerHTML={{ __html: translate('settings.about_desc', lang) }} />
          </div>
        </div>
      </div>
      </div>
    </div>
  );
};

export default SettingsPanel;
