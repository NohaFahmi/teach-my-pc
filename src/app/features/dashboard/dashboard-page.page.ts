import { Component, inject } from '@angular/core';
import { RouterLink } from '@angular/router';
import { DatePipe } from '@angular/common';
import { RuleStoreService } from '../../core/services/rule-store.service';

@Component({
  selector: 'app-dashboard-page',
  imports: [RouterLink, DatePipe],
  templateUrl: './dashboard-page.page.html',
  styleUrl: './dashboard-page.page.scss',
})
export class DashboardPagePage {
  readonly store = inject(RuleStoreService);

  constructor() {
    void this.store.load();
    void this.store.loadActivity();
    void this.store.refreshRuntimeStatuses();
  }

  runningCount(): number {
    return this.store.rules().filter((rule) => rule.enabled && this.store.statusFor(rule.id)?.status === 'Running').length;
  }

  disabledCount(): number { return this.store.rules().filter((rule) => !rule.enabled).length; }

  latestActivities() { return this.store.activities().slice(0, 5); }
  activityTitle(ruleId: string): string { return this.store.rules().find((rule) => rule.id === ruleId)?.title ?? 'Deleted rule'; }
  activitySummary(record: { completedActions: string[]; intendedActions: string[] }): string {
    return record.completedActions.length ? record.completedActions.join(' → ') : record.intendedActions.join(' → ');
  }
}
