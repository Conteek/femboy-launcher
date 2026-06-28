import { X, ChevronDown } from 'lucide-react';
import { useEffect, useRef, useState, type CSSProperties, type ChangeEvent } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { t, type Locale } from './i18n';

export const RAM_STEPS_GIB = [1, 1.5, 2, 2.5, 3, 3.5, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
const MAIN_RAM_TICKS_GIB = new Set([1, 2, 3, 4, 6, 8, 10, 12, 14, 16]);
export type JrePreference = 'recommended' | '8' | '17' | '21' | '25';
export type VersionTypeFilter = 'vanilla' | 'forge' | 'forgeOptifine' | 'fabric' | 'neoforge';

export interface VersionFilterSettings {
  types: Record<VersionTypeFilter, boolean>;
  installedOnly: boolean;
}

const getJreOptions = (): Array<{ value: JrePreference; label: string }> => [
  { value: 'recommended', label: t().jreRecommended },
  { value: '8', label: 'Java 8' },
  { value: '17', label: 'Java 17' },
  { value: '21', label: 'Java 21' },
  { value: '25', label: 'Java 25' },
];

interface SettingsPageProps {
  ramMb: number;
  jrePreference: JrePreference;
  versionFilters: VersionFilterSettings;
  onRamChange: (mb: number) => void;
  onJrePreferenceChange: (preference: JrePreference) => void;
  onVersionFiltersChange: (filters: VersionFilterSettings) => void;
  onLocaleChange: (locale: Locale) => void;
  locale: Locale;
  accentColor: string;
  theme: 'system' | 'light' | 'dark';
  onAccentChange: (color: string) => void;
  onThemeChange: (theme: 'system' | 'light' | 'dark') => void;
  onClose: () => void;
}

const VERSION_FILTER_OPTIONS: Array<{ key: VersionTypeFilter; label: string }> = [
  { key: 'vanilla', label: 'Vanilla' },
  { key: 'forge', label: 'Forge' },
  { key: 'forgeOptifine', label: 'ForgeOptifine' },
  { key: 'fabric', label: 'Fabric' },
  { key: 'neoforge', label: 'NeoForge' },
];

export const DEFAULT_VERSION_FILTERS: VersionFilterSettings = {
  types: {
    vanilla: true,
    forge: true,
    forgeOptifine: true,
    fabric: true,
    neoforge: true,
  },
  installedOnly: false,
};

const ACCENT_PRESETS = [
  { value: '#ffc6b2', label: 'Default' },
  { value: '#ff7db3', label: 'Pink' },
  { value: '#ecdf4b', label: 'Yellow' },
];

export default function SettingsPage({
  ramMb,
  jrePreference,
  versionFilters,
  locale,
  accentColor,
  theme,
  onRamChange,
  onJrePreferenceChange,
  onVersionFiltersChange,
  onLocaleChange,
  onAccentChange,
  onThemeChange,
  onClose,
}: SettingsPageProps) {
  const [version, setVersion] = useState<string>('');
  const [isOpen, setIsOpen] = useState(false);
  const colorInputRef = useRef<HTMLInputElement>(null);
  const isPresetInit = ACCENT_PRESETS.some((p) => p.value.toLowerCase() === accentColor.toLowerCase());
  const [lastCustomColor, setLastCustomColor] = useState<string>(() => {
    return localStorage.getItem('lastCustomColor') || (!isPresetInit ? accentColor : '#888888');
  });
  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  const idx = RAM_STEPS_GIB.reduce(
    (best, g, i) =>
      Math.abs(g * 1024 - ramMb) < Math.abs(RAM_STEPS_GIB[best] * 1024 - ramMb) ? i : best,
    0,
  );

  const fill = `${(idx / (RAM_STEPS_GIB.length - 1)) * 100}%`;
  const typeFiltersDisabled = versionFilters.installedOnly;

  const setTypeFilter = (key: VersionTypeFilter, checked: boolean) => {
    onVersionFiltersChange({
      ...versionFilters,
      types: {
        ...versionFilters.types,
        [key]: checked,
      },
    });
  };

  const setInstalledOnly = (checked: boolean) => {
    onVersionFiltersChange({
      installedOnly: checked,
      types: {
        vanilla: !checked,
        forge: !checked,
        forgeOptifine: !checked,
        fabric: !checked,
        neoforge: !checked,
      },
    });
  };

  const isPreset = ACCENT_PRESETS.some((p) => p.value.toLowerCase() === accentColor.toLowerCase());

  const handleCustomClick = () => {
    if (!isPreset) {
      // Already selected — open picker
      colorInputRef.current?.click();
    } else {
      // Not selected — apply last remembered custom color
      onAccentChange(lastCustomColor);
    }
  };

  const handleCustomColorChange = (e: ChangeEvent<HTMLInputElement>) => {
    const color = e.target.value;
    setLastCustomColor(color);
    localStorage.setItem('lastCustomColor', color);
    onAccentChange(color);
  };

  return (
    <div className="settings-page">
      <div className="settings-header">
        <h2 className="settings-title">{t().settingsTitle}</h2>
        <button className="accounts-close-btn" onClick={onClose} title={t().close}>
          <X size={18} />
        </button>
      </div>

      <div className='settings-content'>

        {/* ── GAME SECTION ──────────────────────────────────────── */}
        <div className="settings-section-label">{t().sectionGame}</div>

        {/* RAM */}
        <div className="setting-block">
          <div className="setting-row-header">
            <span className="setting-name">{t().ram}</span>
            <span className="setting-value">
              <span className="setting-value-number">{RAM_STEPS_GIB[idx] * 1024}</span>
              <span className="setting-value-unit">{t().mib}</span>
            </span>
          </div>

          <input
            type="range"
            className="ram-slider"
            min={0}
            max={RAM_STEPS_GIB.length - 1}
            step={1}
            value={idx}
            onChange={(e) => onRamChange(RAM_STEPS_GIB[Number(e.target.value)] * 1024)}
            style={{ '--fill': fill } as CSSProperties}
          />

          <div className="ram-ticks">
            {RAM_STEPS_GIB.map((g, i) => {
              const isMainTick = MAIN_RAM_TICKS_GIB.has(g);

              return (
                <div
                  key={g}
                  className={`ram-tick ${isMainTick ? 'ram-tick-main' : 'ram-tick-minor'}`}
                  style={{ left: `${(i / (RAM_STEPS_GIB.length - 1)) * 100}%` }}
                >
                  <div className="ram-tick-line" />
                  {isMainTick && <span className="ram-tick-label">{g}</span>}
                </div>
              );
            })}
          </div>
        </div>

        {/* Java / JRE */}
        <div className="setting-block">
          <div className="setting-row-header">
            <span className="setting-name">Java / JRE</span>
          </div>
          <div className="jre-segmented-control" role="radiogroup" aria-label="Java / JRE">
            {getJreOptions().map((option) => (
              <button
                key={option.value}
                type="button"
                className={`jre-option ${jrePreference === option.value ? 'active' : ''}`}
                onClick={() => onJrePreferenceChange(option.value)}
                role="radio"
                aria-checked={jrePreference === option.value}
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>

        {/* Version list */}
        <div className="setting-block">
          <div
            className="setting-row-header"
            style={{ cursor: 'pointer', userSelect: 'none', alignItems: 'center', marginBottom: isOpen ? '14px' : '0px' }}
            onClick={() => setIsOpen(!isOpen)}
          >
            <span className="setting-name">{t().versionList}</span>
            <ChevronDown size={16} className={`dropdown-icon ${isOpen ? 'open' : ''}`} />
          </div>
          {isOpen && (
            <div className="version-filter-list">
              {VERSION_FILTER_OPTIONS.map((option) => (
                <label
                  key={option.key}
                  className={`version-filter-option ${typeFiltersDisabled ? 'disabled' : ''}`}
                >
                  <input
                    type="checkbox"
                    checked={!typeFiltersDisabled && versionFilters.types[option.key]}
                    disabled={typeFiltersDisabled}
                    onChange={(e) => setTypeFilter(option.key, e.target.checked)}
                  />
                  <span>{option.label}</span>
                </label>
              ))}
              <label className="version-filter-option">
                <input
                  type="checkbox"
                  checked={versionFilters.installedOnly}
                  onChange={(e) => setInstalledOnly(e.target.checked)}
                />
                <span>{t().installedOnly}</span>
              </label>
            </div>
          )}
        </div>

        {/* ── APPEARANCE SECTION ─────────────────────────────────── */}
        <div className="settings-section-label">{t().sectionAppearance}</div>

        {/* Language */}
        <div className="setting-block">
          <div className="setting-row-header single">
            <span className="setting-name">{t().language}</span>
            <span className="setting-value">
              <div className="language-segmented-control" role="radiogroup" aria-label={t().language}>
                {([{ value: 'en', label: 'English' }, { value: 'ru', label: 'Русский' }] as { value: Locale; label: string }[]).map((opt) => (
                  <button
                    key={opt.value}
                    type="button"
                    className={`language-option ${locale === opt.value ? 'active' : ''}`}
                    onClick={() => onLocaleChange(opt.value)}
                    role="radio"
                    aria-checked={locale === opt.value}
                  >
                    {opt.label}
                  </button>
                ))}
              </div>
            </span>
          </div>
        </div>

        {/* Theme */}
        <div className="setting-block">
          <div className="setting-row-header single">
            <span className="setting-name">{t().theme}</span>
            <div className="theme-grid">
              {['system', 'light', 'dark'].map((thm) => {
                const isActive = theme === thm;
                return (
                  <button
                    key={thm}
                    type="button"
                    className={`theme-btn ${isActive ? 'active' : ''} theme-btn--${thm}`}
                    onClick={() => onThemeChange(thm as any)}
                    aria-label={thm}
                  >
                    <span className="theme-label">{thm.charAt(0).toUpperCase() + thm.slice(1)}</span>
                  </button>
                );
              })}
            </div>
          </div>
        </div>

        {/* Accent Color */}
        <div className="setting-block">
          <div className="setting-row-header single">
            <span className="setting-name">{t().accentColor}</span>
            <div className="accent-color-grid">
              {ACCENT_PRESETS.map((preset) => {
                const isActive = accentColor.toLowerCase() === preset.value.toLowerCase();
                return (
                  <button
                    key={preset.value}
                    type="button"
                    className={`accent-color-btn ${isActive ? 'active' : ''}`}
                    onClick={() => onAccentChange(preset.value)}
                    aria-label={preset.label}
                    style={!isActive ? { background: preset.value } : undefined}
                  >
                    {isActive && <span className="accent-color-hex">{preset.value}</span>}
                  </button>
                );
              })}

              {/* Custom color button */}
              <button
                type="button"
                className={`accent-color-btn ${!isPreset ? 'active' : 'custom-unselected'}`}
                onClick={handleCustomClick}
                aria-label={t().customColor}
                style={isPreset ? { background: lastCustomColor } : undefined}
              >
                {!isPreset
                  ? <span className="accent-color-hex">{accentColor}</span>
                  : <span
                      className="accent-color-hex accent-color-hex--label"
                      style={{ color: lastCustomColor.toLowerCase() === '#ffffff' ? 'rgba(0,0,0,0.75)' : undefined }}
                    >Custom</span>
                }
                <input
                  ref={colorInputRef}
                  type="color"
                  value={!isPreset ? accentColor : lastCustomColor}
                  onChange={handleCustomColorChange}
                  style={{ position: 'absolute', width: 0, height: 0, opacity: 0, pointerEvents: 'none' }}
                  tabIndex={-1}
                />
              </button>
            </div>
          </div>
        </div>

        {/* Launcher version */}
        <div className="setting-block">
          <div className="setting-row-header single">
            <span className="setting-name">{t().launcherVersion}</span>
            <span className="setting-value">{version}</span>
          </div>
        </div>

      </div>
    </div>
  );
}
