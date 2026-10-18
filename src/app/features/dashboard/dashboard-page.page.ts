import {
  ChangeDetectionStrategy,
  Component,
  computed,
  DestroyRef,
  inject,
  signal,
} from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { PlanPreview, SavedAction, SavedRule } from '../../core/models/interpretation.model';
import { RuleStoreService } from '../../core/services/rule-store.service';

@Component({
  selector: 'app-dashboard-page',
  imports: [RouterLink],
  templateUrl: './dashboard-page.page.html',
  styleUrl: './dashboard-page.page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DashboardPagePage {
  readonly store = inject(RuleStoreService);
  private readonly router = inject(Router);
  private readonly destroyRef = inject(DestroyRef);

  readonly selectedRuleId = signal<string | null>(null);
  readonly selectedRule = computed(
    () => this.store.rules().find((rule) => rule.id === this.selectedRuleId()) ?? null,
  );
  readonly enabledCount = computed(
    () => this.store.rules().filter((rule) => rule.enabled).length,
  );
  readonly runningCount = computed(
    () => this.store.rules().filter(
      (rule) => rule.enabled && this.store.statusFor(rule.id)?.status === 'Running',
    ).length,
  );
  readonly operationError = signal('');
  readonly dryRunRuleId = signal('');
  readonly dryRunLoading = signal(false);
  readonly dryRunResult = signal<PlanPreview | null>(null);
  readonly dryRunError = signal('');

  constructor() {
    void this.store.load();
    void this.store.refreshRuntimeStatuses();
    const refreshHandle = window.setInterval(
      () => void this.store.refreshRuntimeStatuses(),
      1000,
    );
    this.destroyRef.onDestroy(() => window.clearInterval(refreshHandle));
  }

  selectRule(rule: SavedRule): void {
    this.selectedRuleId.set(rule.id);
    this.operationError.set('');
    this.dryRunRuleId.set('');
    this.dryRunResult.set(null);
    this.dryRunError.set('');
  }

  navigateRuleList(event: KeyboardEvent, index: number): void {
    const rules = this.store.rules();
    const nextIndex =
      event.key === 'ArrowDown' ? Math.min(index + 1, rules.length - 1)
        : event.key === 'ArrowUp' ? Math.max(index - 1, 0)
          : event.key === 'Home' ? 0
            : event.key === 'End' ? rules.length - 1
              : -1;
    if (nextIndex < 0) return;

    event.preventDefault();
    const nextRule = rules[nextIndex];
    if (nextRule) this.selectRule(nextRule);
    if (event.currentTarget instanceof HTMLButtonElement) {
      event.currentTarget.parentElement
        ?.querySelectorAll<HTMLButtonElement>('.workbench-rule')
        [nextIndex]?.focus();
    }
  }

  folderName(folderId: string): string {
    return this.store.folders().find((folder) => folder.id === folderId)?.name
      ?? 'Authorized folder';
  }

  actionSummary(actions: SavedAction[]): string {
    return actions.map((action) => this.actionName(action)).join(' · ');
  }

  actionName(action: SavedAction): string {
    switch (action.type) {
      case 'rename': return 'Rename';
      case 'move': return 'Move';
      case 'copy': return 'Copy';
    }
  }

  actionDescription(action: SavedAction): string {
    switch (action.type) {
      case 'rename': return `Rename to ${action.pattern} (keep original extension)`;
      case 'move': return `Move to ${this.folderName(action.destinationFolderId)}`;
      case 'copy': return `Copy to ${this.folderName(action.destinationFolderId)}`;
    }
  }

  async toggle(rule: SavedRule): Promise<void> {
    const enabled = !rule.enabled;
    if (enabled && !window.confirm(
      `Enable “${rule.title}”? It will monitor ${this.folderName(rule.sourceFolderId)} and may rename, move, or copy new files.`,
    )) return;

    this.operationError.set('');
    try {
      await this.store.setEnabled(rule.id, enabled);
    } catch (error: unknown) {
      this.operationError.set(this.errorText(error));
    }
  }

  async dryRun(rule: SavedRule): Promise<void> {
    this.dryRunRuleId.set(rule.id);
    this.dryRunLoading.set(true);
    this.dryRunResult.set(null);
    this.dryRunError.set('');
    try {
      const path = await this.store.choosePreviewFile(rule.id);
      if (!path) {
        this.dryRunRuleId.set('');
        return;
      }
      this.dryRunResult.set(await this.store.preview(rule.id, path));
    } catch (error: unknown) {
      this.dryRunError.set(this.errorText(error));
    } finally {
      this.dryRunLoading.set(false);
    }
  }

  async delete(rule: SavedRule): Promise<void> {
    if (!window.confirm(`Delete “${rule.title}”? This cannot be undone.`)) return;

    this.operationError.set('');
    try {
      await this.store.delete(rule.id);
      if (this.selectedRuleId() === rule.id) this.selectedRuleId.set(null);
    } catch (error: unknown) {
      this.operationError.set(this.errorText(error));
    }
  }

  edit(rule: SavedRule): void {
    void this.router.navigate(['/rules'], { state: { editRuleId: rule.id } });
  }

  private errorText(error: unknown): string {
    if (typeof error === 'string') return error;
    if (error instanceof Error) return error.message;
    if (typeof error === 'object' && error !== null && 'message' in error
      && typeof error.message === 'string') return error.message;
    return 'Unexpected backend error';
  }
}
