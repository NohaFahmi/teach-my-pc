import { Component, DestroyRef, inject } from '@angular/core';
import { DatePipe } from '@angular/common';
import { RuleStoreService } from '../../core/services/rule-store.service';

@Component({
  selector: 'app-activity-page',
  imports: [DatePipe],
  templateUrl: './activity-page.page.html',
  styleUrl: './activity-page.page.scss',
})
export class ActivityPagePage {
  readonly store = inject(RuleStoreService);
  private readonly destroyRef = inject(DestroyRef);

  constructor() {
    void this.store.load();
    void this.store.loadActivity();
    const refreshHandle = window.setInterval(() => void this.store.loadActivity(), 1500);
    this.destroyRef.onDestroy(() => window.clearInterval(refreshHandle));
  }

  ruleTitle(id: string): string {
    return this.store.rules().find((rule) => rule.id === id)?.title ?? 'Deleted rule';
  }

  activitySummary(record: {
    completedActions: string[];
    intendedActions: string[];
    resultingPath: string | null;
  }): string {
    const actions = record.completedActions.length
      ? record.completedActions
      : record.intendedActions;
    if (!record.resultingPath) return actions.join(' → ');
    const segments = record.resultingPath.split(/[\\/]/).filter(Boolean);
    const destination = segments.slice(-2).join('/');
    const lastAction = actions.at(-1) ?? '';
    const verb = /copy/i.test(lastAction) ? 'Copy' : /move/i.test(lastAction) ? 'Move' : '';
    const conciseDestination = verb ? `${verb} to ${destination}` : destination;
    const prefix = actions.slice(0, -1).join(' → ');
    return prefix ? `${prefix} → ${conciseDestination}` : conciseDestination;
  }
}
