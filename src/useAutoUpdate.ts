import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface UpdateInfo {
  version: string;
  url: string;
}

export interface ProgressEvent {
  stage: string;
  current: number;
  total: number;
  message: string;
}

export function useAutoUpdate() {
  const [updateAvailable, setUpdateAvailable] = useState<UpdateInfo | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    async function checkForUpdates() {
      try {
        const update: UpdateInfo | null = await invoke('check_for_updates');
        if (update) {
          setUpdateAvailable(update);
        }
      } catch (err) {
        console.error('Failed to check for updates:', err);
      }
    }

    checkForUpdates();
  }, []);

  useEffect(() => {
    let unlisten: () => void;
    
    async function setupListener() {
        const handler = await listen<ProgressEvent>('download_progress', (event) => {
            const { current, total } = event.payload;
            if (total > 0) {
                const percentage = Math.round((current / total) * 100);
                setProgress(percentage);
            }
        });
        unlisten = handler;
    }
    
    setupListener();

    return () => {
        if (unlisten) unlisten();
    };
  }, []);

  async function performUpdate() {
    if (!updateAvailable) return;

    setIsUpdating(true);
    setProgress(0); // Reset

    try {
      const tempPath: string = await invoke('download_update', { url: updateAvailable.url });
      setProgress(100);

      console.log('Update downloaded to:', tempPath);
      await invoke('apply_update', { tempArchivePath: tempPath });
    } catch (err) {
      console.error('Failed to perform update:', err);
      setIsUpdating(false);
    }
  }

  return { updateAvailable, isUpdating, progress, performUpdate };
}
