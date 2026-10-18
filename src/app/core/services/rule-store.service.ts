import { Injectable, signal } from '@angular/core';
import { invoke, isTauri } from '@tauri-apps/api/core';
import {
  ActivityRecord,
  FolderChoice,
  PlanPreview,
  RuleRuntimeStatus,
  SavedRule,
} from '../models/interpretation.model';

@Injectable({ providedIn: 'root' })
export class RuleStoreService {
  readonly rules = signal<SavedRule[]>([]);
  readonly folders = signal<FolderChoice[]>([]);
  readonly loading = signal(false);
  readonly error = signal('');
  readonly runtimeStatuses = signal<RuleRuntimeStatus[]>([]);
  readonly runtimeError = signal('');
  readonly activities = signal<ActivityRecord[]>([]);
  readonly activityLoading = signal(false);
  readonly activityError = signal('');
  async load(): Promise<void> {
    if (!isTauri()) {
      this.error.set('Saved rules are available in the TeachMyPC desktop app.');
      return;
    }
    this.loading.set(true);
    try {
      const [rules, folders] = await Promise.all([
        invoke<SavedRule[]>('list_rules'),
        invoke<FolderChoice[]>('list_authorized_folders'),
      ]);
      this.rules.set(rules);
      this.folders.set(folders);
      this.error.set('');
    } catch (error: unknown) {
      this.error.set(`Could not load saved rules: ${this.errorMessage(error)}`);
    } finally {
      this.loading.set(false);
    }
  }
  async chooseFolder(): Promise<FolderChoice | null> {
    if (!isTauri()) throw new Error('Folder selection is available in the desktop app.');
    return invoke<FolderChoice | null>('choose_folder');
  }
  async save(rule: SavedRule): Promise<SavedRule> {
    if (!isTauri()) throw new Error('Saving rules is available in the desktop app.');
    const saved = await invoke<SavedRule>('save_rule', { rule });
    await this.load();
    return saved;
  }
  async delete(id: string): Promise<void> {
    if (!isTauri()) throw new Error('Deleting rules is available in the desktop app.');
    await invoke<void>('delete_rule', { id });
    await this.load();
  }
  async setEnabled(id: string, enabled: boolean): Promise<void> {
    await invoke<SavedRule>('set_rule_enabled', { id, enabled });
    await this.load();
    await this.refreshRuntimeStatuses();
  }
  async activity(): Promise<ActivityRecord[]> {
    return invoke<ActivityRecord[]>('list_activity');
  }
  async loadActivity(): Promise<void> {
    if (!isTauri()) {
      this.activityError.set('Activity history is available in the TeachMyPC desktop app.');
      return;
    }
    this.activityLoading.set(true);
    try {
      this.activities.set(await invoke<ActivityRecord[]>('list_activity'));
      this.activityError.set('');
    } catch (error: unknown) {
      this.activityError.set(this.errorMessage(error));
    } finally {
      this.activityLoading.set(false);
    }
  }
  async preview(id: string, samplePath: string): Promise<PlanPreview> {
    return invoke<PlanPreview>('preview_rule', { id, samplePath });
  }
  async choosePreviewFile(id: string): Promise<string | null> {
    return invoke<string | null>('choose_preview_file', { id });
  }
  async refreshRuntimeStatuses(): Promise<void> {
    if (!isTauri()) {
      this.runtimeError.set('Runtime status is available in the desktop app.');
      return;
    }
    try {
      this.runtimeStatuses.set(await invoke<RuleRuntimeStatus[]>('list_rule_runtime_statuses'));
      this.runtimeError.set('');
    } catch (error: unknown) {
      this.runtimeError.set(this.errorMessage(error));
    }
  }
  statusFor(ruleId: string): RuleRuntimeStatus | undefined {
    return this.runtimeStatuses().find((item) => item.ruleId === ruleId);
  }
  private errorMessage(error: unknown): string {
    if (typeof error === 'string') return error;
    if (error instanceof Error) return error.message;
    if (
      typeof error === 'object' &&
      error !== null &&
      'message' in error &&
      typeof error.message === 'string'
    )
      return error.message;
    return 'Unexpected backend error';
  }
}
