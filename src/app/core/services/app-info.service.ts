import { Injectable, signal } from '@angular/core';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { AppInfo, AppInfoState } from '../models/app-info.model';

@Injectable({ providedIn: 'root' })
export class AppInfoService {
  readonly state = signal<AppInfoState>({ status: 'loading' });

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
}
