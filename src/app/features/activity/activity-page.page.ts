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

  actions(record: { completedActions: string[]; intendedActions: string[] }): string {
    return record.completedActions.length ? record.completedActions.join(' → ') : record.intendedActions.join(' → ');
  }
}
