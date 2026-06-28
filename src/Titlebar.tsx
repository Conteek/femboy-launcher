import { useState, useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Minus, Square, X, Copy, Settings, Maximize2, Minimize2, Folder } from 'lucide-react';
import { Command } from '@tauri-apps/plugin-shell';
import { invoke } from '@tauri-apps/api/core';
import { t } from './i18n';

const appWindow = getCurrentWindow();

// Configuration: 
// - If FORCE_MACOS_STYLE is true, it forces that style.
// - If null, it auto-detects based on navigator.platform
const FORCE_MACOS_STYLE: boolean | null = null;

// Simple OS detection
const isMacOS = () => {
  if (FORCE_MACOS_STYLE !== null) return FORCE_MACOS_STYLE;
  return navigator.platform.toUpperCase().indexOf('MAC') >= 0;
};

const openFolder = async () => {
  const path: string = await invoke('get_game_dir_path');

  const platform = navigator.platform.toUpperCase();
  if (platform.indexOf('WIN') >= 0) {
    // Correct way to open a folder in Windows explorer via shell is often `explorer.exe <path>`
    // The previous `explorer` command might be interpreted wrongly by Tauri shell plugin.
    // Try explicitly using `explorer.exe` and the path as an argument.
    await Command.create('explorer.exe', [path]).execute();
  } else {
    await Command.create('open', [path]).execute();
  }
};

interface TitlebarProps {
  onToggleSettings?: () => void;
}

function TitlebarDefault({ onToggleSettings, isMaximized, handleMinimize, handleMaximize, handleClose }: any) {
  return (
    <div data-tauri-drag-region className="titlebar">
      <div data-tauri-drag-region className="titlebar-drag-region">
        <img src="/logo.png" alt="Icon" data-tauri-drag-region />
        <span data-tauri-drag-region>Femboy Launcher</span>
      </div>
      <div className="titlebar-actions">
        <button className="titlebar-button" onClick={openFolder} title={t().gameFolder}>
          <Folder size={15} />
        </button>
        <button className="titlebar-button" onClick={onToggleSettings} title={t().settings}>
          <Settings size={15} />
        </button>
        <button className="titlebar-button" onClick={handleMinimize}>
          <Minus size={16} />
        </button>
        <button className="titlebar-button" onClick={handleMaximize}>
          {isMaximized ? <Copy size={14} /> : <Square size={14} />}
        </button>
        <button className="titlebar-button close" onClick={handleClose}>
          <X size={16} />
        </button>
      </div>
    </div>
  );
}

function TitlebarMac({ onToggleSettings, isMaximized, handleMinimize, handleMaximize, handleClose }: any) {
  return (
    <div data-tauri-drag-region className="titlebar titlebar-mac">
      <div className="titlebar-actions-mac">
        <button className="mac-btn mac-close" onClick={handleClose}><X size={10} /></button>
        <button className="mac-btn mac-minimize" onClick={handleMinimize}><Minus size={10} /></button>
        <button className="mac-btn mac-maximize" onClick={handleMaximize}>
          {isMaximized ? <Minimize2 size={10} /> : <Maximize2 size={10} />}
        </button>
        <button className="mac-btn mac-settings" onClick={onToggleSettings}><Settings size={10} /></button>
        <button className="mac-btn mac-folder" onClick={openFolder}><Folder size={10} /></button>
      </div>
      <div data-tauri-drag-region className="titlebar-title-mac">
        Femboy Launcher
      </div>
      <div className="titlebar-spacer-mac" />
    </div>
  );
}

export default function Titlebar({ onToggleSettings }: TitlebarProps) {
  const [isMaximized, setIsMaximized] = useState(false);
  const [useMacStyle, setUseMacStyle] = useState(false);

  useEffect(() => {
    setUseMacStyle(isMacOS());

    appWindow.isMaximized().then(setIsMaximized);
    const unlisten = appWindow.onResized(async () => {
      const maximized = await appWindow.isMaximized();
      setIsMaximized(maximized);
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  const handleMinimize = () => appWindow.minimize();
  const handleMaximize = async () => {
    const maximized = await appWindow.isMaximized();
    if (maximized) {
      await appWindow.unmaximize();
      setIsMaximized(false);
    } else {
      await appWindow.maximize();
      setIsMaximized(true);
    }
  };
  const handleClose = () => appWindow.close();

  const props = { onToggleSettings, isMaximized, handleMinimize, handleMaximize, handleClose };

  return useMacStyle
    ? <TitlebarMac {...props} />
    : <TitlebarDefault {...props} />;
}
