import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAutoUpdate } from './useAutoUpdate';
import Titlebar from './Titlebar';
import SettingsPage, {
  DEFAULT_VERSION_FILTERS,
  type JrePreference,
  type VersionFilterSettings,
  type VersionTypeFilter,
} from './SettingsPage';
import RadialProgress from './RadialProgress';
import { ChevronDown, X, Plus, User, Settings2, AlertCircle, CheckCircle, RefreshCw, Package, Pencil, Trash2, FolderOpen, Image, Download, Layers, ChevronLeft, ChevronRight } from 'lucide-react';
import { open, save } from '@tauri-apps/plugin-dialog';
import { t, setLocale, getLocale, type Locale } from './i18n';
import confetti from 'canvas-confetti';
import StatisticsPanel from './StatisticsPanel';

// ── Types ─────────────────────────────────────────────────────────────────────

interface VersionInfo {
  id: string;
  installed: boolean;
  isModpack?: boolean;
  modpackId?: string;
  modpackVersion?: string;
  modpackGameDir?: string;
}

interface ModpackMeta {
  id: string;
  name: string;
  description?: string;
  author?: string;
  version: string;
  avatar?: string;
  created_at: string;
}

interface ModMeta {
  id: string;
  name: string;
  author: string | null;
  icon: string | null;
}

function parseVersionFilters(value?: string): VersionFilterSettings {
  if (!value) return DEFAULT_VERSION_FILTERS;

  try {
    const parsed = JSON.parse(value) as Partial<VersionFilterSettings>;
    return {
      installedOnly: parsed.installedOnly === true,
      types: {
        ...DEFAULT_VERSION_FILTERS.types,
        ...(parsed.types ?? {}),
      },
    };
  } catch {
    return DEFAULT_VERSION_FILTERS;
  }
}

function getVersionType(id: string): VersionTypeFilter {
  if (id.startsWith('ForgeOptifine ')) return 'forgeOptifine';
  if (id.startsWith('Forge ')) return 'forge';
  if (id.startsWith('Fabric ')) return 'fabric';
  if (id.startsWith('Neoforge ') || id.startsWith('NeoForge ')) return 'neoforge';
  return 'vanilla';
}

function filterVersions(versions: VersionInfo[], filters: VersionFilterSettings): VersionInfo[] {
  if (filters.installedOnly) return versions.filter((version) => version.installed);
  return versions.filter((version) => filters.types[getVersionType(version.id)]);
}

export type Account =
  | { type: 'Offline'; username: string }
  | { type: 'Ely'; username: string; uuid: string; access_token: string }
  | { type: 'Microsoft'; username: string; uuid: string; access_token: string };

export function getAccountName(acc: Account): string {
  return acc.username;
}

export function isElyAccount(acc: Account): boolean {
  return acc.type === 'Ely';
}

export function isMicrosoftAccount(acc: Account): boolean {
  return acc.type === 'Microsoft';
}

export function isSameAccount(a: Account | null, b: Account | null): boolean {
  if (!a || !b) return a === b;
  if (a.type !== b.type) return false;
  return a.username === b.username;
}

function versionDisplayName(id: string): string {
  if (
    id.startsWith('Forge ') ||
    id.startsWith('ForgeOptifine ') ||
    id.startsWith('Fabric ') ||
    id.startsWith('NeoForge ') ||
    id.startsWith('Neoforge ')
  ) {
    const pipeIdx = id.indexOf(' | ');
    if (pipeIdx !== -1) return id.slice(0, pipeIdx);
  }
  return id;
}

interface ProgressEvent {
  stage: string;
  current: number;
  total: number;
  message: string;
}

// ── Version Dropdown ──────────────────────────────────────────────────────────

interface VersionDropdownProps {
  versions: VersionInfo[];
  selected: VersionInfo | null;
  onSelect: (v: VersionInfo) => void;
  disabled?: boolean;
}

function VersionDropdown({ versions, selected, onSelect, disabled }: VersionDropdownProps) {
  const [isOpen, setIsOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setIsOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

  useEffect(() => {
    if (isOpen && listRef.current && selected) {
      const el = listRef.current.querySelector<HTMLLIElement>('.dropdown-item.selected');
      if (el) el.scrollIntoView({ block: 'nearest' });
    }
  }, [isOpen, selected]);

  if (versions.length === 0) {
    return (
      <div className="custom-dropdown">
        <div className="dropdown-header dropdown-header--loading">
          <span className="dropdown-selected" style={{ opacity: 0.5 }}>{t().loadingVersions}</span>
        </div>
      </div>
    );
  }

  return (
    <div className={`custom-dropdown ${disabled ? 'custom-dropdown--disabled' : ''}`} ref={ref}>
      <div className="dropdown-header" onClick={() => !disabled && setIsOpen(!isOpen)}>
        <span className="dropdown-selected" style={{ opacity: selected ? (selected.installed ? 1 : 0.6) : 0.5 }}>
          {selected ? versionDisplayName(selected.id) : t().selectVersion}
        </span>
        <ChevronDown size={16} className={`dropdown-icon ${isOpen ? 'open' : ''}`} />
      </div>
      {isOpen && (
        <div className="dropdown-list-container dropdown-list-container--up">
          <ul className="dropdown-list" ref={listRef}>
            {versions.map((v) => {
              const isSelected = v.id === selected?.id;
              const opacity = v.installed ? 1 : 0.6;
              return (
                <li
                  key={v.id}
                  className={`dropdown-item ${isSelected ? 'selected' : ''}`}
                  onClick={() => { onSelect(v); setIsOpen(false); }}
                >
                  <span style={{ opacity }}>{versionDisplayName(v.id)}</span>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
  );
}

// ── Account Dropdown ──────────────────────────────────────────────────────────

interface AccountDropdownProps {
  accounts: Account[];
  selectedAccount: Account | null;
  onSelectAccount: (acc: Account) => void;
  onOpenManage: () => void;
  disabled?: boolean;
}

function AccountDropdown({ accounts, selectedAccount, onSelectAccount, onOpenManage, disabled }: AccountDropdownProps) {
  const [isOpen, setIsOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setIsOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

  return (
    <div className={`custom-dropdown ${disabled ? 'custom-dropdown--disabled' : ''}`} ref={ref}>
      <div className="dropdown-header" onClick={() => !disabled && setIsOpen(!isOpen)}>
        {selectedAccount ? (
          <span className="dropdown-selected">
            {isElyAccount(selectedAccount) && (
              <img src="/elyby.svg" width={16} height={16} style={{ flexShrink: 0 }} alt="Ely.by" />
            )}
            {isMicrosoftAccount(selectedAccount) && (
              <img src="/microsoft.svg" width={16} height={16} style={{ flexShrink: 0 }} alt="Microsoft" />
            )}
            {getAccountName(selectedAccount)}
          </span>
        ) : (
          <span className="dropdown-selected dropdown-selected--placeholder">{t().noAccounts}</span>
        )}
        <ChevronDown size={16} className={`dropdown-icon ${isOpen ? 'open' : ''}`} />
      </div>

      {isOpen && (
        <div className="dropdown-list-container dropdown-list-container--up">
          <ul className="dropdown-list">
            {accounts.length === 0 && (
              <li className="dropdown-item dropdown-item--empty">{t().noAccounts}</li>
            )}
            {accounts.map((acc, i) => {
              const name = getAccountName(acc);
              const isSelected = isSameAccount(selectedAccount, acc);
              return (
                <li
                  key={i}
                  className={`dropdown-item ${isSelected ? 'selected' : ''}`}
                  onClick={() => { onSelectAccount(acc); setIsOpen(false); }}
                >
                  <div style={{ display: 'flex', alignItems: 'center' }}>
                    {isElyAccount(acc) ? (
                      <img src="/elyby.svg" width={16} height={16} style={{ marginRight: 8, flexShrink: 0 }} alt="Ely.by" />
                    ) : isMicrosoftAccount(acc) ? (
                      <img src="/microsoft.svg" width={16} height={16} style={{ marginRight: 8, flexShrink: 0 }} alt="Microsoft" />
                    ) : (
                      <User size={13} style={{ marginRight: 8, opacity: 0.6, flexShrink: 0 }} />
                    )}
                    {name}
                  </div>
                </li>
              );
            })}
          </ul>
          <div className="dropdown-divider" />
          <div className="dropdown-manage-item" onClick={() => { setIsOpen(false); onOpenManage(); }}>
            <Settings2 size={13} style={{ marginRight: 8, flexShrink: 0 }} />
            {t().manageAccounts}
          </div>
        </div>
      )}
    </div>
  );
}

// ── AccountsPage ──────────────────────────────────────────────────────────────

interface AccountsPageProps {
  accounts: Account[];
  onClose: () => void;
  onAddOffline: (name: string) => void;
  onAddEly: () => void;
  onAddMicrosoft: () => void;
  onRemove: (acc: Account) => void;
}

function AccountsPage({ accounts, onClose, onAddOffline, onAddEly, onAddMicrosoft, onRemove }: AccountsPageProps) {
  const [showInput, setShowInput] = useState(false);
  const [showOnlineDropdown, setShowOnlineDropdown] = useState(false);
  const [nickname, setNickname] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (showInput) inputRef.current?.focus();
  }, [showInput]);

  const handleAddOffline = () => {
    const trimmed = nickname.trim();
    if (trimmed && !accounts.some(a => a.type === 'Offline' && a.username === trimmed)) {
      onAddOffline(trimmed);
    }
    setNickname('');
    setShowInput(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleAddOffline();
    if (e.key === 'Escape') { setShowInput(false); setNickname(''); }
  };

  return (
    <div className="accounts-page">
      <div className="accounts-header">
        <h2 className="accounts-title">{t().accountsTitle}</h2>
        <button className="accounts-close-btn" onClick={onClose} title={t().close}>
          <X size={18} />
        </button>
      </div>

      <div className="accounts-list">
        {accounts.length === 0 && !showInput && (
          <div className="accounts-empty">{t().noAccountsAdded}</div>
        )}
        {accounts.map((acc, i) => {
          const name = getAccountName(acc);
          return (
            <div className="account-item" key={`${acc.type}:${name}-${i}`}>
              <div className="account-item-icon">
                {isElyAccount(acc) ? (
                  <img src="/elyby.svg" width={16} height={16} alt="Ely.by" />
                ) : isMicrosoftAccount(acc) ? (
                  <img src="/microsoft.svg" width={16} height={16} alt="Microsoft" />
                ) : (
                  <User size={16} color="currentColor" />
                )}
              </div>
              <span className="account-item-name">{name}</span>
              <button className="account-item-remove" onClick={() => onRemove(acc)} title={t().removeAccount}>
                <X size={14} />
              </button>
            </div>
          );
        })}

        {showInput && (
          <div className="account-item account-item--input">
            <div className="account-item-icon"><User size={16} /></div>
            <input
              ref={inputRef}
              className="account-input"
              placeholder={t().enterNickname}
              value={nickname}
              onChange={e => setNickname(e.target.value)}
              onKeyDown={handleKeyDown}
              maxLength={32}
            />
            <button className="account-item-confirm" onClick={handleAddOffline}>OK</button>
            <button className="account-item-remove" onClick={() => { setShowInput(false); setNickname(''); }} title={t().cancel}>
              <X size={14} />
            </button>
          </div>
        )}
      </div>

      {!showInput && (
        <div style={{ display: 'flex', gap: '8px' }}>
          <button className="accounts-add-btn" onClick={() => setShowInput(true)}>
            <Plus size={18} />
            <span>{t().addOffline}</span>
          </button>

          <div style={{ position: 'relative', width: '100%' }}>
            <button className="accounts-add-btn" style={{ background: 'var(--accent)', borderColor: '#daaa98ff', color: '#000000ff' }} onClick={() => setShowOnlineDropdown(!showOnlineDropdown)}>
              <Plus size={18} />
              <span>{t().addOnline}</span>
            </button>
            {showOnlineDropdown && (
              <div style={{ position: 'absolute', bottom: '100%', left: 0, marginBottom: '4px', background: '#242222', border: '1px solid var(--border)', borderRadius: '8px', padding: '4px', display: 'flex', flexDirection: 'column', minWidth: '200px', zIndex: 10, boxShadow: '0 4px 12px rgba(0,0,0,0.2)' }}>
                <div className="dropdown-item" style={{ padding: '8px', borderRadius: '4px' }} onClick={() => { setShowOnlineDropdown(false); onAddMicrosoft(); }}>
                  <img src="/microsoft.svg" width={16} height={16} style={{ marginRight: 8 }} alt="Microsoft" />
                  <span>{t().loginMicrosoft}</span>
                </div>
                <div className="dropdown-item" style={{ padding: '8px', borderRadius: '4px' }} onClick={() => { setShowOnlineDropdown(false); onAddEly(); }}>
                  <img src="/elyby.svg" width={16} height={16} style={{ marginRight: 8 }} alt="Ely.by" />
                  <span>{t().loginElyBy}</span>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Modpack Components ────────────────────────────────────────────────────────

const SNAPSHOT_FOLDERS = [
  { key: 'mods', label: 'mods' },
  { key: 'resourcepacks', label: 'resourcepacks' },
  { key: 'shaderpacks', label: 'shaderpacks' },
  { key: 'config', label: 'config' },
  { key: 'saves', label: 'saves' },
  { key: 'screenshots', label: 'screenshots' },
  { key: 'texturepacks', label: 'texturepacks' },
];

interface AvatarPickerProps {
  value: string | null;
  onChange: (dataUrl: string) => void;
}

function AvatarPicker({ value, onChange }: AvatarPickerProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [dragging, setDragging] = useState(false);

  const processImage = (file: File) => {
    const reader = new FileReader();
    reader.onload = (ev) => {
      const img = new window.Image();
      img.onload = () => {
        const size = Math.min(img.width, img.height, 1000);
        const canvas = canvasRef.current!;
        canvas.width = size;
        canvas.height = size;
        const ctx = canvas.getContext('2d')!;
        const sx = (img.width - size) / 2;
        const sy = (img.height - size) / 2;
        ctx.drawImage(img, sx, sy, size, size, 0, 0, size, size);
        onChange(canvas.toDataURL('image/png'));
      };
      img.src = ev.target?.result as string;
    };
    reader.readAsDataURL(file);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragging(false);
    const file = e.dataTransfer.files[0];
    if (file && file.type.startsWith('image/')) processImage(file);
  };

  return (
    <div className="avatar-picker-wrap">
      <canvas ref={canvasRef} style={{ display: 'none' }} />
      <div
        className={`avatar-picker ${dragging ? 'avatar-picker--drag' : ''}`}
        onClick={() => inputRef.current?.click()}
        onDragOver={e => { e.preventDefault(); setDragging(true); }}
        onDragLeave={() => setDragging(false)}
        onDrop={handleDrop}
      >
        {value ? (
          <img src={value} alt="avatar" className="avatar-picker-img" />
        ) : (
          <div className="avatar-picker-placeholder">
            <Image size={28} style={{ opacity: 0.4 }} />
            <span>{t().clickOrDragImage}</span>
          </div>
        )}
      </div>
      <input
        ref={inputRef}
        type="file"
        accept="image/*"
        style={{ display: 'none' }}
        onChange={e => {
          const file = e.target.files?.[0];
          if (file) processImage(file);
        }}
      />
    </div>
  );
}

interface CreateModpackPageProps {
  versions: VersionInfo[];
  onClose: () => void;
  onCreate: (meta: ModpackMeta) => void;
}

function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    || 'modpack';
}

function CreateModpackPage({ versions, onClose, onCreate }: CreateModpackPageProps) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [author, setAuthor] = useState('');
  const [version, setVersion] = useState(versions.find(v => !v.isModpack) ?? null);
  const [avatar, setAvatar] = useState<string | null>(null);
  const [mode, setMode] = useState<'blank' | 'snapshot'>('blank');
  const [snapshotFolders, setSnapshotFolders] = useState<string[]>(['mods', 'config']);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const versionDropRef = useRef<HTMLDivElement>(null);
  const [versionOpen, setVersionOpen] = useState(false);
  useEffect(() => {
    const h = (e: MouseEvent) => { if (versionDropRef.current && !versionDropRef.current.contains(e.target as Node)) setVersionOpen(false); };
    document.addEventListener('mousedown', h);
    return () => document.removeEventListener('mousedown', h);
  }, []);

  const realVersions = versions.filter(v => !v.isModpack);

  const toggleFolder = (key: string) => {
    setSnapshotFolders(prev =>
      prev.includes(key) ? prev.filter(f => f !== key) : [...prev, key]
    );
  };

  const handleCreate = async () => {
    if (!name.trim()) { setError(t().nameRequired); return; }
    if (!version) { setError(t().versionRequired); return; }
    setSaving(true);
    setError('');
    try {
      const id = slugify(name.trim()) + '-' + Date.now();
      await invoke('create_modpack', {
        id,
        name: name.trim(),
        description: description.trim() || null,
        author: author.trim() || null,
        version: version.id,
        mode,
        snapshotFolders,
      });
      if (avatar) {
        await invoke('save_modpack_avatar', { id, imageData: avatar });
      }
      const newMeta: ModpackMeta = {
        id,
        name: name.trim(),
        description: description.trim() || undefined,
        author: author.trim() || undefined,
        version: version.id,
        avatar: avatar ? 'avatar.png' : undefined,
        created_at: new Date().toISOString(),
      };
      onCreate(newMeta);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="create-modpack-page">
      <div className="modpack-page-header">
        <h2 className="modpack-page-title">{t().createModpack}</h2>
        <button className="accounts-close-btn" onClick={onClose}><X size={18} /></button>
      </div>
      <div className="create-modpack-body">
        <div className="create-modpack-left">
          <AvatarPicker value={avatar} onChange={setAvatar} />
          <div className="modpack-mode-selector">
            <button
              className={`mode-btn ${mode === 'blank' ? 'active' : ''}`}
              onClick={() => setMode('blank')}
            >{t().blankMode}</button>
            <button
              className={`mode-btn ${mode === 'snapshot' ? 'active' : ''}`}
              onClick={() => setMode('snapshot')}
            >{t().snapshotMode}</button>
          </div>
          {mode === 'snapshot' && (
            <div className="snapshot-folders version-filter-list">
              <span className="snapshot-label">{t().foldersToCopy}</span>
              {SNAPSHOT_FOLDERS.map(f => (
                <label key={f.key} className="version-filter-option">
                  <input
                    type="checkbox"
                    checked={snapshotFolders.includes(f.key)}
                    onChange={() => toggleFolder(f.key)}
                  />
                  <span>{f.label}</span>
                </label>
              ))}
            </div>
          )}
        </div>
        <div className="create-modpack-right">
          <div className="modpack-field">
            <label className="modpack-label">{t().nameLabel} <span style={{ color: 'var(--accent)' }}>*</span></label>
            <input className="modpack-input" value={name} onChange={e => setName(e.target.value)} placeholder={t().namePlaceholder} maxLength={64} />
          </div>
          <div className="modpack-field">
            <label className="modpack-label">{t().descriptionLabel}</label>
            <textarea className="modpack-input modpack-textarea" value={description} onChange={e => setDescription(e.target.value)} placeholder={t().descriptionPlaceholder} rows={3} />
          </div>
          <div className="modpack-field">
            <label className="modpack-label">{t().authorLabel}</label>
            <input className="modpack-input" value={author} onChange={e => setAuthor(e.target.value)} placeholder={t().descriptionPlaceholder} maxLength={64} />
          </div>
          <div className="modpack-field">
            <label className="modpack-label">{t().versionLabel} <span style={{ color: 'var(--accent)' }}>*</span></label>
            <div className="custom-dropdown" ref={versionDropRef}>
              <div className="dropdown-header" onClick={() => setVersionOpen(o => !o)}>
                <span className="dropdown-selected" style={{ opacity: version ? 1 : 0.5 }}>
                  {version ? versionDisplayName(version.id) : t().selectVersion}
                </span>
                <ChevronDown size={16} className={`dropdown-icon ${versionOpen ? 'open' : ''}`} />
              </div>
              {versionOpen && (
                <div className="dropdown-list-container dropdown-list-container--up">
                  <ul className="dropdown-list">
                    {realVersions.map(v => (
                      <li key={v.id} className={`dropdown-item ${v.id === version?.id ? 'selected' : ''}`}
                        onClick={() => { setVersion(v); setVersionOpen(false); }}>
                        <span style={{ opacity: v.installed ? 1 : 0.6 }}>{versionDisplayName(v.id)}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </div>
          {error && <div className="modpack-error">{error}</div>}
          <div className="create-modpack-actions">
            <button className="play-button play-button--install" onClick={handleCreate} disabled={saving} style={{ fontSize: 14, padding: '10px 24px', letterSpacing: 1 }}>
              {saving ? t().creating : t().create}
            </button>
            <button className="modpack-cancel-btn" onClick={onClose}>{t().cancel}</button>
          </div>
        </div>
      </div>
    </div>
  );
}

interface EditModpackPageProps {
  modpack: ModpackMeta;
  versions: VersionInfo[];
  onClose: () => void;
  onSave: (updated: ModpackMeta) => void;
}

function EditModpackPage({ modpack, versions, onClose, onSave }: EditModpackPageProps) {
  const [name, setName] = useState(modpack.name);
  const [description, setDescription] = useState(modpack.description ?? '');
  const [author, setAuthor] = useState(modpack.author ?? '');
  const [version, setVersion] = useState<VersionInfo | null>(
    versions.find(v => v.id === modpack.version) ?? null
  );
  const [avatar, setAvatar] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const versionDropRef = useRef<HTMLDivElement>(null);
  const [versionOpen, setVersionOpen] = useState(false);
  useEffect(() => {
    const h = (e: MouseEvent) => { if (versionDropRef.current && !versionDropRef.current.contains(e.target as Node)) setVersionOpen(false); };
    document.addEventListener('mousedown', h);
    return () => document.removeEventListener('mousedown', h);
  }, []);

  // Load existing avatar
  useEffect(() => {
    if (modpack.avatar) {
      invoke<string | null>('get_modpack_avatar', { id: modpack.id }).then(v => {
        if (v) setAvatar(v);
      }).catch(() => { });
    }
  }, [modpack.id, modpack.avatar]);

  const realVersions = versions.filter(v => !v.isModpack);

  const handleSave = async () => {
    if (!name.trim()) { setError(t().nameRequired); return; }
    if (!version) { setError(t().versionRequired); return; }
    setSaving(true);
    setError('');
    try {
      await invoke('update_modpack', {
        id: modpack.id,
        name: name.trim(),
        description: description.trim() || null,
        author: author.trim() || null,
        version: version.id,
      });
      if (avatar && !avatar.startsWith('data:image') === false) {
        // only save if avatar changed (new selection)
        if (avatar !== null && modpack.avatar ? avatar !== 'keep' : true) {
          await invoke('save_modpack_avatar', { id: modpack.id, imageData: avatar });
        }
      }
      const updated: ModpackMeta = {
        ...modpack,
        name: name.trim(),
        description: description.trim() || undefined,
        author: author.trim() || undefined,
        version: version.id,
        avatar: avatar ? 'avatar.png' : modpack.avatar,
      };
      onSave(updated);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="create-modpack-page">
      <div className="modpack-page-header">
        <h2 className="modpack-page-title">{t().editModpack}</h2>
        <button className="accounts-close-btn" onClick={onClose}><X size={18} /></button>
      </div>
      <div className="create-modpack-body">
        <div className="create-modpack-left">
          <AvatarPicker value={avatar} onChange={setAvatar} />
        </div>
        <div className="create-modpack-right">
          <div className="modpack-field">
            <label className="modpack-label">{t().nameLabel} <span style={{ color: 'var(--accent)' }}>*</span></label>
            <input className="modpack-input" value={name} onChange={e => setName(e.target.value)} placeholder={t().namePlaceholder} maxLength={64} />
          </div>
          <div className="modpack-field">
            <label className="modpack-label">{t().descriptionLabel}</label>
            <textarea className="modpack-input modpack-textarea" value={description} onChange={e => setDescription(e.target.value)} placeholder={t().descriptionPlaceholder} rows={3} />
          </div>
          <div className="modpack-field">
            <label className="modpack-label">{t().authorLabel}</label>
            <input className="modpack-input" value={author} onChange={e => setAuthor(e.target.value)} placeholder={t().descriptionPlaceholder} maxLength={64} />
          </div>
          <div className="modpack-field">
            <label className="modpack-label">{t().versionLabel} <span style={{ color: 'var(--accent)' }}>*</span></label>
            <div className="custom-dropdown" ref={versionDropRef}>
              <div className="dropdown-header" onClick={() => setVersionOpen(o => !o)}>
                <span className="dropdown-selected" style={{ opacity: version ? 1 : 0.5 }}>
                  {version ? versionDisplayName(version.id) : t().selectVersion}
                </span>
                <ChevronDown size={16} className={`dropdown-icon ${versionOpen ? 'open' : ''}`} />
              </div>
              {versionOpen && (
                <div className="dropdown-list-container dropdown-list-container--up">
                  <ul className="dropdown-list">
                    {realVersions.map(v => (
                      <li key={v.id} className={`dropdown-item ${v.id === version?.id ? 'selected' : ''}`}
                        onClick={() => { setVersion(v); setVersionOpen(false); }}>
                        <span style={{ opacity: v.installed ? 1 : 0.6 }}>{versionDisplayName(v.id)}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </div>
          {error && <div className="modpack-error">{error}</div>}
          <div className="create-modpack-actions">
            <button className="play-button play-button--install" onClick={handleSave} disabled={saving} style={{ fontSize: 14, padding: '10px 24px', letterSpacing: 1 }}>
              {saving ? t().saving : t().save}
            </button>
            <button className="modpack-cancel-btn" onClick={onClose}>{t().cancel}</button>
          </div>
        </div>
      </div>
    </div>
  );
}

interface ModpacksManagerPageProps {
  modpacks: ModpackMeta[];
  versions: VersionInfo[];
  onClose: () => void;
  onRefresh: () => void;
}

function ModpacksManagerPage({ modpacks, versions, onClose, onRefresh }: ModpacksManagerPageProps) {
  const [showAddDropdown, setShowAddDropdown] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [editingPack, setEditingPack] = useState<ModpackMeta | null>(null);
  const [avatars, setAvatars] = useState<Record<string, string>>({});
  const addDropRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const h = (e: MouseEvent) => { if (addDropRef.current && !addDropRef.current.contains(e.target as Node)) setShowAddDropdown(false); };
    document.addEventListener('mousedown', h);
    return () => document.removeEventListener('mousedown', h);
  }, []);

  useEffect(() => {
    modpacks.forEach(mp => {
      if (mp.avatar && !avatars[mp.id]) {
        invoke<string | null>('get_modpack_avatar', { id: mp.id })
          .then(v => { if (v) setAvatars(a => ({ ...a, [mp.id]: v })); })
          .catch(() => { });
      }
    });
  }, [modpacks]);

  const handleDelete = async (mp: ModpackMeta) => {
    if (!confirm(t().deleteModpackConfirm(mp.name))) return;
    try {
      await invoke('delete_modpack', { id: mp.id });
      onRefresh();
    } catch (e) {
      alert(t().errorDeletingModpack + e);
    }
  };

  const handleOpenFolder = async (mp: ModpackMeta) => {
    try {
      await invoke('open_modpack_folder', { id: mp.id });
    } catch (e) {
      alert('Error: ' + e);
    }
  };

  const handleExport = async (mp: ModpackMeta) => {
    try {
      const filePath = await save({
        filters: [{ name: 'Tar GZ', extensions: ['tar.gz'] }],
        defaultPath: `${mp.name}.tar.gz`
      });
      if (filePath) {
        await invoke('export_modpack', { id: mp.id, destPath: filePath });
        alert(t().modpackExported);
      }
    } catch (e) {
      alert('Error: ' + e);
    }
  };

  const handleImport = async () => {
    try {
      const filePath = await open({
        filters: [{ name: 'Tar GZ', extensions: ['tar.gz'] }],
        multiple: false
      });
      if (filePath) {
        await invoke('import_modpack', { archivePath: filePath });
        alert(t().modpackImported);
        onRefresh();
      }
    } catch (e) {
      alert(e);
    }
  };

  if (showCreate) {
    return (
      <CreateModpackPage
        versions={versions}
        onClose={() => setShowCreate(false)}
        onCreate={() => { onRefresh(); setShowCreate(false); }}
      />
    );
  }

  if (editingPack) {
    return (
      <EditModpackPage
        modpack={editingPack}
        versions={versions}
        onClose={() => setEditingPack(null)}
        onSave={() => { onRefresh(); setEditingPack(null); }}
      />
    );
  }

  return (
    <div className="modpacks-manager-page">
      <div className="modpack-page-header">
        <h2 className="modpack-page-title">{t().modpacksTitle}</h2>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <div style={{ position: 'relative' }} ref={addDropRef}>
            <button className="accounts-add-btn" style={{ marginTop: 0, padding: '8px 16px', minWidth: 140 }}
              onClick={() => setShowAddDropdown(d => !d)}>
              <Plus size={16} />
              <span>{t().addModpack}</span>
              <ChevronDown size={14} className={`dropdown-icon ${showAddDropdown ? 'open' : ''}`} style={{ marginLeft: 'auto' }} />
            </button>
            {showAddDropdown && (
              <div className="modpack-add-dropdown">
                <div className="dropdown-item" onClick={() => { setShowAddDropdown(false); handleImport(); }}>
                  <Package size={14} style={{ marginRight: 8, opacity: 0.7 }} />
                  {t().importModpack}
                </div>
                <div className="dropdown-item" onClick={() => { setShowAddDropdown(false); setShowCreate(true); }}>
                  <Plus size={14} style={{ marginRight: 8, opacity: 0.7 }} />
                  {t().createModpack}
                </div>
              </div>
            )}
          </div>
          <button className="accounts-close-btn" onClick={onClose}><X size={18} /></button>
        </div>
      </div>

      <div className="modpacks-manager-list">
        {modpacks.length === 0 && (
          <div className="accounts-empty">{t().noModpacks}</div>
        )}
        {modpacks.map(mp => (
          <div className="modpack-manager-item" key={mp.id}>
            <div className="modpack-manager-avatar">
              {avatars[mp.id] ? (
                <img src={avatars[mp.id]} alt={mp.name} />
              ) : (
                <Package size={20} style={{ opacity: 0.4 }} />
              )}
            </div>
            <div className="modpack-manager-info">
              <span className="modpack-manager-name">{mp.name}</span>
              <span className="modpack-manager-meta">
                {versionDisplayName(mp.version)}
                {mp.author && <> · {mp.author}</>}
              </span>
            </div>
            <div className="modpack-manager-actions">
              <button className="modpack-icon-btn" title={t().export} onClick={() => handleExport(mp)}>
                <Download size={15} />
              </button>
              <button className="modpack-icon-btn" title={t().openFolder} onClick={() => handleOpenFolder(mp)}>
                <FolderOpen size={15} />
              </button>
              <button className="modpack-icon-btn" title={t().edit} onClick={() => setEditingPack(mp)}>
                <Pencil size={15} />
              </button>
              <button className="modpack-icon-btn modpack-icon-btn--danger" title={t().delete} onClick={() => handleDelete(mp)}>
                <Trash2 size={15} />
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

interface ModpackCardProps {
  modpack: ModpackMeta;
  isSelected: boolean;
  onClick: () => void;
}

function ModpackCard({ modpack, isSelected, onClick }: ModpackCardProps) {
  const [avatar, setAvatar] = useState<string | null>(null);

  useEffect(() => {
    if (modpack.avatar) {
      invoke<string | null>('get_modpack_avatar', { id: modpack.id })
        .then(v => { if (v) setAvatar(v); })
        .catch(() => { });
    }
  }, [modpack.id, modpack.avatar]);

  return (
    <div
      className={`modpack-card ${isSelected ? 'modpack-card--selected' : ''}`}
      onClick={onClick}
    >
      <div className="modpack-card-avatar-large">
        {avatar ? (
          <img src={avatar} alt={modpack.name} />
        ) : (
          <Package size={40} style={{ opacity: 0.45 }} />
        )}
      </div>
      {isSelected && (
        <div className="modpack-card-info-below">
          <span className="modpack-card-name-below" title={modpack.name}>{modpack.name}</span>
          <span className="modpack-card-version-below">{versionDisplayName(modpack.version)}</span>
        </div>
      )}
    </div>
  );
}

interface ModpacksListProps {
  modpacks: ModpackMeta[];
  selectedModpackId: string | null;
  onSelect: (mp: ModpackMeta) => void;
  onOpenManager: () => void;
}

interface ModpackDetailsPanelProps {
  modpack: ModpackMeta;
}

function ModpackDetailsPanel({ modpack }: ModpackDetailsPanelProps) {
  const [mods, setMods] = useState<ModMeta[]>([]);
  const [loading, setLoading] = useState(false);
  const [avatar, setAvatar] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    invoke<ModMeta[]>('get_modpack_mods', { id: modpack.id })
      .then(setMods)
      .catch(console.error)
      .finally(() => setLoading(false));

    if (modpack.avatar) {
      invoke<string | null>('get_modpack_avatar', { id: modpack.id })
        .then(setAvatar)
        .catch(console.error);
    } else {
      setAvatar(null);
    }
  }, [modpack.id, modpack.avatar]);

  return (
    <div className="modpack-details-panel">
      <div className="modpack-details-header">
        <div className="modpack-details-icon-wrapper">
          {avatar ? <img src={avatar} alt="icon" /> : <Package size={32} opacity={0.6} />}
        </div>
        <div className="modpack-details-info">
          <h3>{modpack.name}</h3>
          <span className="modpack-details-version">{versionDisplayName(modpack.version)}</span>
          <span className="modpack-details-author">{(modpack.author && modpack.author.trim() !== '') ? modpack.author : t().unknownAuthor}</span>
        </div>
      </div>
      {modpack.description && (
        <div className="modpack-details-desc">{modpack.description}</div>
      )}
      <div className="modpack-details-mods-header">
        <h4>{t().modlist}</h4>
        <span className="modpack-details-mods-count">{t().modsCount(mods.length)}</span>
      </div>
      <div className="modpack-details-mods-list">
        {loading ? (
          <div className="modpack-details-mods-loading">{t().loadingMods}</div>
        ) : mods.length === 0 ? (
          <div className="modpack-details-mods-loading" style={{ opacity: 0.5 }}>{t().noModsFound}</div>
        ) : (
          mods.map((mod, idx) => (
            <div className="mod-item" key={`${mod.id}-${idx}`}>
              <div className="mod-item-icon">
                {mod.icon ? <img src={mod.icon} alt={mod.name} /> : <Package size={16} opacity={0.4} />}
              </div>
              <div className="mod-item-info">
                <span className="mod-item-name">{mod.name}</span>
                {mod.author && <span className="mod-item-author">{mod.author}</span>}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function ModpacksList({ modpacks, selectedModpackId, onSelect, onOpenManager }: ModpacksListProps) {
  const selectedModpack = modpacks.find(mp => mp.id === selectedModpackId);
  const listRef = useRef<HTMLDivElement>(null);

  const scrollLeft = () => {
    if (listRef.current) {
      listRef.current.scrollBy({ left: -166, behavior: 'smooth' });
    }
  };

  const scrollRight = () => {
    if (listRef.current) {
      listRef.current.scrollBy({ left: 166, behavior: 'smooth' });
    }
  };

  return (
    <div className="modpacks-list-container">
      <div className="modpack-page-header" style={{ marginBottom: 16, display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <h2 className="modpack-page-title">{t().modpacksTitle}</h2>
          <button className="modpack-icon-btn manage-modpacks-btn" onClick={onOpenManager}>
            <Settings2 size={18} />
          </button>
        </div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <button className="modpack-nav-btn" onClick={scrollLeft}>
            <ChevronLeft size={18} />
          </button>
          <button className="modpack-nav-btn" onClick={scrollRight}>
            <ChevronRight size={18} />
          </button>
        </div>
      </div>
      <div className="modpacks-list-layout">
        <div className="modpacks-list" ref={listRef}>
          {modpacks.map(mp => (
            <ModpackCard
              key={mp.id}
              modpack={mp}
              isSelected={selectedModpackId === mp.id}
              onClick={() => onSelect(mp)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

// ── Mod Manager Modal ───────────────────────────────────────────────────────────

interface ModFileInfo {
  filename: string;
  enabled: boolean;
  meta: {
    id: string;
    name: string;
    author: string | null;
    icon: string | null;
  } | null;
}

interface ModManagerModalProps {
  modpackId: string;
  onClose: () => void;
}

function ModManagerModal({ modpackId, onClose }: ModManagerModalProps) {
  const [mods, setMods] = useState<ModFileInfo[]>([]);
  const [loading, setLoading] = useState(true);

  const loadMods = async () => {
    setLoading(true);
    try {
      const result = await invoke<ModFileInfo[]>('list_mods', { modpackId });
      setMods(result);
    } catch (e) {
      console.error('Failed to load mods:', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadMods(); }, [modpackId]);

  const handleToggle = async (mod: ModFileInfo) => {
    try {
      const newFilename = await invoke<string>('toggle_mod', { modpackId, filename: mod.filename });
      setMods(prev => prev.map(m =>
        m.filename === mod.filename
          ? { ...m, filename: newFilename, enabled: !m.enabled }
          : m
      ));
    } catch (e) {
      console.error('Failed to toggle mod:', e);
    }
  };

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) onClose();
  };

  const displayName = (mod: ModFileInfo) => mod.meta?.name ?? mod.filename.replace(/\.jar(\.disabled)?$/, '');

  return (
    <div className="mod-manager-overlay" onClick={handleOverlayClick}>
      <div className="mod-manager-modal">
        <div className="mod-manager-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <h2 className="mod-manager-title">{t().modManagerTitle}</h2>
            {!loading && (
              <span className="mod-manager-count">{t().modsCount(mods.length)}</span>
            )}
          </div>
          <button className="accounts-close-btn" onClick={onClose} title={t().close}>
            <X size={18} />
          </button>
        </div>
        <div className="mod-manager-list">
          {loading ? (
            <div className="mod-manager-empty">{t().modManagerLoading}</div>
          ) : mods.length === 0 ? (
            <div className="mod-manager-empty">{t().modManagerNoMods}</div>
          ) : (
            mods.map((mod) => (
              <div
                key={mod.filename}
                className={`mod-manager-item${mod.enabled ? '' : ' mod-manager-item--disabled'}`}
                onClick={() => handleToggle(mod)}
              >
                <div className="mod-manager-icon">
                  {mod.meta?.icon
                    ? <img src={mod.meta.icon} alt={displayName(mod)} />
                    : <Package size={18} opacity={0.4} />}
                </div>
                <div className="mod-manager-info">
                  <span className="mod-manager-name">{displayName(mod)}</span>
                  {mod.meta?.author && (
                    <span className="mod-manager-filename">{mod.meta.author}</span>
                  )}
                  <span className="mod-manager-filename" style={{ opacity: 0.45 }}>
                    {mod.filename}
                  </span>
                </div>
                <label className="mod-toggle" onClick={e => e.stopPropagation()}>
                  <input
                    type="checkbox"
                    checked={mod.enabled}
                    onChange={() => handleToggle(mod)}
                  />
                  <span className="mod-toggle-track" />
                </label>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}

// ── Progress Bar ──────────────────────────────────────────────────────────────

interface ProgressBarProps {
  progress: ProgressEvent | null;
  label?: string;
  pct?: number;
}

function ProgressBar({ progress, label, pct: passedPct }: ProgressBarProps) {
  let pct = passedPct ?? 0;
  let message = label ?? '';

  if (progress) {
    pct = progress.total > 0 ? Math.round((progress.current / progress.total) * 100) : 0;
    message = progress.message;
  }

  return (
    <div className="progress-container">
      <div className="progress-message">{message}</div>
      <div className="progress-track">
        <div
          className="progress-fill"
          style={{ width: `${pct}%` }}
        />
      </div>
      <div className="progress-pct">{pct}%</div>
    </div>
  );
}

// ── Update Modal ──────────────────────────────────────────────────────────────

function UpdateModal({ progress }: { progress: number }) {
  return (
    <div className="modal-overlay">
      <div className="update-modal">
        <h2 className="update-title">{t().updatingLauncher}</h2>
        <RadialProgress value={progress} />
      </div>
    </div>
  );
}

// ── App ───────────────────────────────────────────────────────────────────────

type LauncherState = 'idle' | 'loading_versions' | 'installing' | 'launching' | 'error' | 'success';

function App() {
  const { updateAvailable, isUpdating, progress: updateProgress, performUpdate } = useAutoUpdate();

  const handleImageClick = (e: React.MouseEvent<HTMLImageElement>) => {
    const audio = new Audio('/nya.mp3');
    audio.play().catch(err => console.error("Error playing nya:", err));

    const x = e.clientX / window.innerWidth;
    const y = e.clientY / window.innerHeight;

    confetti({
      particleCount: 100,
      spread: 70,
      origin: { x, y },
      zIndex: 9999
    });
  };
  const [showAccounts, setShowAccounts] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showModpacksManager, setShowModpacksManager] = useState(false);
  const [showModManager, setShowModManager] = useState(false);
  const [modpacks, setModpacks] = useState<ModpackMeta[]>([]);
  const [selectedModpackId, setSelectedModpackId] = useState<string | null>(null);
  const [ramMb, setRamMb] = useState(2048);
  const [jrePreference, setJrePreference] = useState<JrePreference>('recommended');
  const [locale, setLocaleState] = useState<Locale>(getLocale());
  const [accentColor, setAccentColor] = useState<string>('#FFC6B2');
  const [theme, setTheme] = useState<'system' | 'light' | 'dark'>('system');
  const [versionFilters, setVersionFilters] = useState<VersionFilterSettings>(DEFAULT_VERSION_FILTERS);
  const settingsRef = useRef<Record<string, string>>({});
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [selectedAccount, setSelectedAccount] = useState<Account | null>(null);

  const [versions, setVersions] = useState<VersionInfo[]>([]);
  const [selectedVersion, setSelectedVersion] = useState<VersionInfo | null>(null);
  const [state, setState] = useState<LauncherState>('loading_versions');
  const [errorMsg, setErrorMsg] = useState('');
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const filteredVersions = useMemo(
    () => filterVersions(versions, versionFilters),
    [versions, versionFilters],
  );

  useEffect(() => {
    const handleGlobalContextMenu = (e: MouseEvent) => {
      e.preventDefault();
    };

    document.addEventListener('contextmenu', handleGlobalContextMenu);

    return () => {
      document.removeEventListener('contextmenu', handleGlobalContextMenu);
    };
  }, []);

  useEffect(() => {
    const unlisten = listen('modpack-download-progress', (event) => {
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useEffect(() => {
    const init = async () => {
      try {
        const settings: Record<string, string> = await invoke('load_settings');
        settingsRef.current = settings ?? {};
        const savedRam = parseInt(settings?.ramMb ?? '', 10);
        if (!isNaN(savedRam)) setRamMb(savedRam);
        if (['recommended', '8', '17', '21', '25'].includes(settings?.javaPreference ?? '')) {
          setJrePreference(settings.javaPreference as JrePreference);
        }
        if (['ru', 'en'].includes(settings?.locale ?? '')) {
          setLocale(settings.locale as Locale);
          setLocaleState(settings.locale as Locale);
        }
        if (settings?.accentColor) {
          setAccentColor(settings.accentColor);
          document.documentElement.style.setProperty('--accent', settings.accentColor);
        }
        if (settings?.theme) {
          setTheme(settings.theme as 'system' | 'light' | 'dark');
        }
        setVersionFilters(parseVersionFilters(settings?.versionFilters));
        const savedModpackId = settings?.selectedModpackId ?? null;
        if (savedModpackId) setSelectedModpackId(savedModpackId);

        const savedVersionId = settings?.selectedVersion ?? null;
        loadVersions(savedVersionId ?? undefined);
      } catch {
        loadVersions();
      }
      loadAccounts();
      loadModpacks();
      invoke('set_rpc_activity', {
        details: t().rpcInLauncher,
        state: t().rpcBrowsingMain,
      }).catch(() => { });
    };
    init();
  }, []);

  // Listen for download progress events from Rust
  useEffect(() => {
    const unlisten = listen<ProgressEvent>('download_progress', (event) => {
      setProgress(event.payload);
      if (event.payload.stage === 'done') {
        // Refresh installed status
        setTimeout(() => loadVersions(), 500);
        setTimeout(() => {
          setState('idle');
          setProgress(null);
        }, 1500);
      }
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  // Listen for game_closed to reset RPC
  useEffect(() => {
    const unlisten = listen('game_closed', () => {
      invoke('set_rpc_activity', {
        details: t().rpcInLauncher,
        state: t().rpcBrowsingMain,
      }).catch(() => { });
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  const loadVersions = async (savedVersionId?: string) => {
    try {
      const versionList = await invoke<VersionInfo[]>('get_versions');
      setVersions(versionList);

      if (versionList.length === 0) {
        setErrorMsg(t().errorLoadVersionsOffline);
        setState('error');
      } else {
        if (state === 'loading_versions' || state === 'error') setState('idle');
      }

      setSelectedVersion(prev => {
        // First load: restore saved version or default to first
        if (prev === null) {
          if (savedVersionId) {
            const found = versionList.find(v => v.id === savedVersionId);
            if (found) return found;
          }
          return versionList.length > 0 ? versionList[0] : null;
        }
        // Subsequent loads: refresh installed status of current selection
        if (prev.isModpack) {
          return {
            ...prev,
            installed: versionList.some(v => v.id === prev.modpackVersion && v.installed)
          };
        }
        const refreshed = versionList.find(v => v.id === prev.id);
        return refreshed ?? prev;
      });
    } catch (e) {
      setErrorMsg(`${t().errorLoadVersionsPrefix}${e}`);
      setState('error');
    }
  };

  const loadModpacks = async () => {
    try {
      const list = await invoke<ModpackMeta[]>('list_modpacks');
      setModpacks(list);
      // If we have a selectedModpackId saved, set it as selectedVersion once loaded
      if (selectedModpackId) {
        const mp = list.find(m => m.id === selectedModpackId);
        if (mp) {
          setSelectedVersion(prev => {
            if (prev?.isModpack && prev.modpackId === mp.id) return prev;
            return {
              id: mp.name,
              installed: versions.some(v => v.id === mp.version && v.installed),
              isModpack: true,
              modpackId: mp.id,
              modpackVersion: mp.version,
            };
          });
        }
      }
    } catch (e) {
      console.error('Failed to load modpacks:', e);
    }
  };

  const loadAccounts = async () => {
    try {
      const loaded: Account[] = await invoke('load_accounts');
      setAccounts(loaded);

      const savedId = settingsRef.current?.selectedAccountId;
      if (savedId && loaded.length > 0) {
        const found = loaded.find(a => `${a.type}:${a.username}` === savedId);
        if (found) {
          setSelectedAccount(found);
          return;
        }
      }

      if (loaded.length > 0) setSelectedAccount(loaded[0]);
    } catch (e) {
      console.error('Failed to load accounts:', e);
    }
  };

  const addOfflineAccount = async (name: string) => {
    const newAcc: Account = { type: 'Offline', username: name };
    const newAccounts = [...accounts, newAcc];
    setAccounts(newAccounts);
    if (selectedAccount === null) {
      setSelectedAccount(newAcc);
      saveSetting('selectedAccountId', `${newAcc.type}:${newAcc.username}`);
    }
    try {
      await invoke('save_accounts', { accounts: newAccounts });
    } catch (e) {
      console.error('Failed to save accounts:', e);
    }
  };

  const addElyAccount = async () => {
    try {
      setState('installing'); // Reuse loading state for UI
      setProgress({ stage: 'start', current: 0, total: 1, message: t().waitingBrowserAuth });

      const newAcc: Account = await invoke('login_ely');

      // Remove any existing account with the same type and username to replace it
      const filtered = accounts.filter(a => !isSameAccount(a, newAcc));
      const newAccounts = [...filtered, newAcc];

      setAccounts(newAccounts);
      setSelectedAccount(newAcc);
      saveSetting('selectedAccountId', `${newAcc.type}:${newAcc.username}`);
      await invoke('save_accounts', { accounts: newAccounts });

      setState('idle');
      setProgress(null);
    } catch (e) {
      setErrorMsg(`${t().errorElyByPrefix}${e}`);
      setState('error');
      setProgress(null);
    }
  };

  const addMicrosoftAccount = async () => {
    try {
      setState('installing');
      setProgress({ stage: 'start', current: 0, total: 1, message: t().waitingBrowserAuth });

      const newAcc: Account = await invoke('login_microsoft');

      const filtered = accounts.filter(a => !isSameAccount(a, newAcc));
      const newAccounts = [...filtered, newAcc];

      setAccounts(newAccounts);
      setSelectedAccount(newAcc);
      saveSetting('selectedAccountId', `${newAcc.type}:${newAcc.username}`);
      await invoke('save_accounts', { accounts: newAccounts });

      setState('idle');
      setProgress(null);
    } catch (e) {
      setErrorMsg(`${t().errorMicrosoftPrefix}${e}`);
      setState('error');
      setProgress(null);
    }
  };

  const removeAccount = async (target: Account) => {
    const newAccounts = accounts.filter(a => !isSameAccount(a, target));
    setAccounts(newAccounts);
    if (isSameAccount(selectedAccount, target)) {
      const nextAcc = newAccounts.length > 0 ? newAccounts[0] : null;
      setSelectedAccount(nextAcc);
      saveSetting('selectedAccountId', nextAcc ? `${nextAcc.type}:${nextAcc.username}` : '');
    }
    try {
      await invoke('save_accounts', { accounts: newAccounts });
    } catch (e) {
      console.error('Failed to save accounts:', e);
    }
  };

  // Merge a single key into settings so other saved keys are not overwritten
  const saveSetting = useCallback((key: string, value: string) => {
    settingsRef.current = { ...settingsRef.current, [key]: value };
    invoke('save_settings', { settings: settingsRef.current }).catch(() => { });
  }, []);

  const handleAccountSelect = useCallback((acc: Account | null) => {
    setSelectedAccount(acc);
    if (acc) {
      saveSetting('selectedAccountId', `${acc.type}:${acc.username}`);
    } else {
      saveSetting('selectedAccountId', '');
    }
  }, [saveSetting]);

  const handleVersionSelect = useCallback((v: VersionInfo) => {
    setSelectedVersion(v);
    setState('idle');
    setErrorMsg('');
    if (v.isModpack && v.modpackId) {
      setSelectedModpackId(v.modpackId);
      saveSetting('selectedModpackId', v.modpackId);
    } else {
      setSelectedModpackId(null);
      saveSetting('selectedModpackId', '');
      saveSetting('selectedVersion', v.id);
    }
  }, [saveSetting]);

  const handleRamChange = useCallback((mb: number) => {
    setRamMb(mb);
    saveSetting('ramMb', String(mb));
  }, [saveSetting]);

  const handleJrePreferenceChange = useCallback((preference: JrePreference) => {
    setJrePreference(preference);
    saveSetting('javaPreference', preference);
  }, [saveSetting]);

  const handleVersionFiltersChange = useCallback((filters: VersionFilterSettings) => {
    setVersionFilters(filters);
    saveSetting('versionFilters', JSON.stringify(filters));
  }, [saveSetting]);

  const handleLocaleChange = useCallback((newLocale: Locale) => {
    setLocale(newLocale);
    setLocaleState(newLocale);
    saveSetting('locale', newLocale);
  }, [saveSetting]);

  const handleAccentChange = useCallback((color: string) => {
    setAccentColor(color);
    document.documentElement.style.setProperty('--accent', color);
    saveSetting('accentColor', color);
  }, [saveSetting]);

  const handleThemeChange = useCallback((newTheme: 'system' | 'light' | 'dark') => {
    setTheme(newTheme);
    saveSetting('theme', newTheme);
  }, [saveSetting]);

  useEffect(() => {
    const applyTheme = (t: 'system' | 'light' | 'dark') => {
      const isLight = t === 'light' || (t === 'system' && window.matchMedia('(prefers-color-scheme: light)').matches);
      if (isLight) {
        document.documentElement.setAttribute('data-theme', 'light');
      } else {
        document.documentElement.removeAttribute('data-theme');
      }
    };

    applyTheme(theme);

    if (theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: light)');
      const handler = () => applyTheme('system');
      mediaQuery.addEventListener('change', handler);
      return () => mediaQuery.removeEventListener('change', handler);
    }
  }, [theme]);

  useEffect(() => {
    if (versions.length === 0) return;
    if (selectedVersion && (selectedVersion.isModpack || filteredVersions.some((version) => version.id === selectedVersion.id))) return;

    const nextVersion = filteredVersions[0] ?? null;
    setSelectedVersion(nextVersion);
    if (nextVersion) saveSetting('selectedVersion', nextVersion.id);
  }, [filteredVersions, saveSetting, selectedVersion, versions.length]);

  const handleInstall = async () => {
    if (!selectedVersion) return;
    setState('installing');
    setErrorMsg('');
    setProgress({ stage: 'start', current: 0, total: 1, message: t().startingInstall });
    try {
      await invoke('install_version', { versionId: selectedVersion.id });
    } catch (e) {
      setErrorMsg(`${t().errorInstallPrefix}${e}`);
      setState('error');
      setProgress(null);
    }
  };

  const handlePlay = async () => {
    if (!selectedVersion || !selectedAccount) return;
    setState('launching');
    setErrorMsg('');
    try {
      // Resolve the real version ID and optional game dir for modpack
      const realVersionId = selectedVersion.isModpack && selectedVersion.modpackVersion
        ? selectedVersion.modpackVersion
        : selectedVersion.id;

      let gameDirOverride: string | null = null;
      if (selectedVersion.isModpack && selectedVersion.modpackId) {
        gameDirOverride = await invoke<string>('get_modpack_path', { id: selectedVersion.modpackId });
      }

      await invoke('launch_minecraft', {
        versionId: realVersionId,
        account: selectedAccount,
        ramMb,
        javaPreference: jrePreference,
        gameDirOverride,
      });

      // RPC: in-game status
      invoke('set_rpc_activity', {
        details: t().rpcInGame,
        state: t().rpcPlayingOn(versionDisplayName(selectedVersion.id)),
      }).catch(() => { });

      setTimeout(() => setState('idle'), 2000);
    } catch (e) {
      setErrorMsg(`${t().errorLaunchPrefix}${e}`);
      setState('error');
    }
  };

  // Build the effective version list: real versions + virtual modpack entry (if selected)
  const selectedModpack = modpacks.find(mp => mp.id === selectedModpackId) ?? null;
  const modpackVirtualVersion: VersionInfo | null = selectedModpack
    ? {
      id: selectedModpack.name,
      installed: filteredVersions.some(v => v.id === selectedModpack.version && v.installed),
      isModpack: true,
      modpackId: selectedModpack.id,
      modpackVersion: selectedModpack.version,
    }
    : null;

  const versionsWithModpack: VersionInfo[] = modpackVirtualVersion
    ? [modpackVirtualVersion, ...filteredVersions]
    : filteredVersions;

  const isInstalled = selectedVersion?.installed ?? false;
  const isBusy = state === 'installing' || state === 'launching' || state === 'loading_versions';
  const canPlay = isInstalled && !!selectedAccount && state === 'idle';
  const canInstall = !isInstalled && !!selectedVersion && state === 'idle';

  // Render action button
  const renderActionButton = () => {
    if (state === 'loading_versions') {
      return <button className="play-button play-button--loading" disabled>{t().loading}</button>;
    }
    if (state === 'installing') {
      return <button className="play-button play-button--installing" disabled>{t().installing}</button>;
    }
    if (state === 'launching') {
      return <button className="play-button play-button--launching" disabled>{t().launching}</button>;
    }
    if (!selectedVersion) {
      return <button className="play-button" disabled><span className='play-button-text'>{t().play}</span></button>;
    }
    if (!isInstalled) {
      return (
        <button
          className="play-button play-button--install"
          onClick={handleInstall}
          disabled={!canInstall}
        >
          <div className="icon" />
          <span className='play-button-text'>{t().install}</span>
        </button>
      );
    }
    return (
      <button
        className="play-button"
        onClick={handlePlay}
        disabled={!canPlay}
        title={!selectedAccount ? t().selectAccount : ''}
      >
        <img src="/play.svg" alt="Logo" width={16} height={16} style={{ marginRight: 6 }} />
        <span className='play-button-text'>{t().play}</span>
      </button>
    );
  };

  return (
    <>
      {isUpdating && <UpdateModal progress={updateProgress} />}

      <Titlebar onToggleSettings={() => { setShowSettings(s => !s); setShowAccounts(false); setShowModpacksManager(false); }} />
      <div className="app-container">

        <div className="main-content">

          <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
            {showSettings ? (
              <div className="content-area">
                <SettingsPage
                  ramMb={ramMb}
                  jrePreference={jrePreference}
                  versionFilters={versionFilters}
                  locale={locale}
                  accentColor={accentColor}
                  theme={theme}
                  onRamChange={handleRamChange}
                  onJrePreferenceChange={handleJrePreferenceChange}
                  onVersionFiltersChange={handleVersionFiltersChange}
                  onLocaleChange={handleLocaleChange}
                  onAccentChange={handleAccentChange}
                  onThemeChange={handleThemeChange}
                  onClose={() => setShowSettings(false)}
                />
              </div>
            ) : showAccounts ? (
              <div className="content-area">
                <AccountsPage
                  accounts={accounts}
                  onClose={() => setShowAccounts(false)}
                  onAddOffline={addOfflineAccount}
                  onAddEly={addElyAccount}
                  onAddMicrosoft={addMicrosoftAccount}
                  onRemove={removeAccount}
                />
              </div>
            ) : showModpacksManager ? (
              <div className="content-area">
                <ModpacksManagerPage
                  modpacks={modpacks}
                  versions={filteredVersions}
                  onClose={() => setShowModpacksManager(false)}
                  onRefresh={loadModpacks}
                />
              </div>
            ) : (
              <div className="content-area">
                <ModpacksList
                  modpacks={modpacks}
                  selectedModpackId={selectedModpackId}
                  onSelect={(mp) => {
                    // Toggle selection
                    if (selectedModpackId === mp.id) {
                      setSelectedModpackId(null);
                      saveSetting('selectedModpackId', '');
                      // Restore first regular version
                      const first = filteredVersions[0] ?? null;
                      setSelectedVersion(first);
                      if (first) saveSetting('selectedVersion', first.id);
                    } else {
                      setSelectedModpackId(mp.id);
                      saveSetting('selectedModpackId', mp.id);
                      // Set virtual modpack version as selected
                      const virt: VersionInfo = {
                        id: mp.name,
                        installed: filteredVersions.some(v => v.id === mp.version && v.installed),
                        isModpack: true,
                        modpackId: mp.id,
                        modpackVersion: mp.version,
                      };
                      setSelectedVersion(virt);
                    }
                  }}
                  onOpenManager={() => setShowModpacksManager(true)}
                />
                
                <StatisticsPanel />
              </div>
            )}
            {selectedModpack && !showSettings && !showAccounts && !showModpacksManager && (
              <div className="global-modpack-details">
                <ModpackDetailsPanel modpack={selectedModpack} />
              </div>
            )}
          </div>

          <div className="bottom-container">
            {/* Progress bar shown during install */}
            {(state === 'installing' || state === 'launching') && progress && (
              <ProgressBar progress={progress} />
            )}

            {/* Error message */}
            {state === 'error' && errorMsg && (
              <div className="error-banner">
                <AlertCircle size={14} style={{ flexShrink: 0 }} />
                <span>{errorMsg}</span>
                <button className="error-dismiss" onClick={() => setState('idle')}>
                  <X size={12} />
                </button>
              </div>
            )}

            {/* Success / launching message */}
            {state === 'launching' && (
              <div className="success-banner">
                <CheckCircle size={14} style={{ flexShrink: 0 }} />
                <span>{t().minecraftLaunching}</span>
              </div>
            )}

            <div className='bottom-section'>
              {updateAvailable && !isUpdating && (
                <button className="update-bar" onClick={performUpdate}>
                  <div className='update-bar-content'>
                    <RefreshCw size={16} />
                    <span>{t().updateAvailable}: {updateAvailable.version}</span>
                  </div>
                </button>
              )}
              <div className="bottom-bar">
                <div className="selectors">
                  <AccountDropdown
                    accounts={accounts}
                    selectedAccount={selectedAccount}
                    onSelectAccount={handleAccountSelect}
                    onOpenManage={() => setShowAccounts(true)}
                    disabled={isBusy}
                  />
                  <VersionDropdown
                    versions={versionsWithModpack}
                    selected={selectedVersion}
                    onSelect={handleVersionSelect}
                    disabled={isBusy}
                  />
                  {selectedVersion && isInstalled && (selectedVersion.isModpack || getVersionType(selectedVersion.id) !== 'vanilla') ? (
                    <button
                      className="mod-manager-btn"
                      onClick={() => setShowModManager(true)}
                      title={t().modManager}
                      disabled={isBusy}
                      style={isBusy ? { opacity: 0.45, pointerEvents: 'none' } : {}}
                    >
                      <Layers size={15} />
                    </button>
                  ) : null}
                </div>
                {renderActionButton()}
              </div>
            </div>
          </div>

        </div>
      </div>
      {showModManager && selectedVersion && (selectedVersion.isModpack || getVersionType(selectedVersion.id) !== 'vanilla') && (
        <ModManagerModal
          modpackId={selectedVersion.isModpack && selectedVersion.modpackId ? selectedVersion.modpackId : 'global'}
          onClose={() => setShowModManager(false)}
        />
      )}
    </>
  );
}

export default App;
