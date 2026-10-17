import { Injectable, signal } from '@angular/core';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { AppInfo, AppInfoState } from '../models/app-info.model';

@Injectable({ providedIn: 'root' })
export class AppInfoService {
  readonly state = signal<AppInfoState>({ status: 'loading' });
  readonly localAi = signal<{ status: 'loading' | 'available' | 'unavailable' | 'model-unavailable'; endpoint?: string; model?: string; message?: string }>({ status: 'loading' });

  async load(): Promise<void> {
    if (!isTauri()) {
      this.state.set({ status: 'unavailable', message: 'App information is available in the desktop app.' });
      return;
    }

    this.state.set({ status: 'loading' });
    try {
      const info = await invoke<AppInfo>('get_app_info');
      this.state.set({ status: 'available', info });
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : typeof error === 'string' ? error : 'The desktop service could not be reached.';
      this.state.set({ status: 'unavailable', message });
    }
  }

  async checkLocalAi(): Promise<void> {
    if (!isTauri()) { this.localAi.set({ status: 'unavailable', message: 'Connection checks are available in the desktop app.' }); return; }
    this.localAi.set({ status: 'loading' });
    try {
      const status = await invoke<{ endpoint: string; model: string; connected: boolean; modelAvailable: boolean }>('get_local_ai_status');
      if (!status.connected) {
        this.localAi.set({ status: 'unavailable', endpoint: status.endpoint, model: status.model, message: 'Ollama is offline or did not return a valid health response. Start Ollama, then retry.' });
      } else if (!status.modelAvailable) {
        this.localAi.set({ status: 'model-unavailable', endpoint: status.endpoint, model: status.model, message: `Ollama is online, but ${status.model} is not installed. Run ollama pull ${status.model}, then retry.` });
      } else {
        this.localAi.set({ status: 'available', endpoint: status.endpoint, model: status.model });
      }
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : typeof error === 'string' ? error : 'The local AI status check failed.';
      this.localAi.set({ status: 'unavailable', message });
    }
  }
}
