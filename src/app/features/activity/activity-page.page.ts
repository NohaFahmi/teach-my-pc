import { ChangeDetectionStrategy, Component, DestroyRef, computed, inject, signal } from '@angular/core';
import { DatePipe } from '@angular/common';
import { RuleStoreService } from '../../core/services/rule-store.service';
import { ActivityRecord } from '../../core/models/interpretation.model';

type OutcomeFilter = 'all' | 'success' | 'skipped' | 'failed' | 'partial_failure';
interface ActivityViewRecord { record: ActivityRecord; key: string; }
interface ActivityGroup { key: string; label: string; events: ActivityViewRecord[]; }

@Component({
  selector: 'app-activity-page',
  imports: [DatePipe],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './activity-page.page.html',
  styleUrl: './activity-page.page.scss',
})
export class ActivityPagePage {
  readonly store = inject(RuleStoreService);
  private readonly destroyRef = inject(DestroyRef);
  readonly filter = signal<OutcomeFilter>('all');
  readonly expanded = signal<ReadonlySet<string>>(new Set());
  readonly filters: { id: OutcomeFilter; label: string }[] = [
    { id: 'all', label: 'All' }, { id: 'success', label: 'Completed' },
    { id: 'skipped', label: 'Skipped' }, { id: 'failed', label: 'Failed' },
    { id: 'partial_failure', label: 'Partial failure' },
  ];
  readonly groups = computed(() => {
    const seen = new Map<string, number>();
    const records = this.store.activities().map((record) => {
      const signature = this.eventKey(record);
      const occurrence = seen.get(signature) ?? 0;
      seen.set(signature, occurrence + 1);
      return { record, key: `${signature}|${occurrence}` };
    })
      .filter(({ record }) => this.filter() === 'all' || record.status === this.filter())
      .sort((a, b) => b.record.timestamp - a.record.timestamp);
    const grouped = new Map<string, ActivityGroup>();
    for (const event of records) {
      const date = new Date(event.record.timestamp * 1000);
      const key = `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
      let group = grouped.get(key);
      if (!group) {
        group = { key, label: this.dateLabel(date), events: [] };
        grouped.set(key, group);
      }
      group.events.push(event);
    }
    return [...grouped.values()];
  });

  constructor() {
    void this.store.load();
    void this.store.loadActivity();
    const refreshHandle = window.setInterval(() => void this.store.loadActivity(), 1500);
    this.destroyRef.onDestroy(() => window.clearInterval(refreshHandle));
  }

  ruleTitle(id: string): string {
    return this.store.rules().find((rule) => rule.id === id)?.title ?? 'Deleted rule';
  }

  setFilter(filter: OutcomeFilter): void { this.filter.set(filter); }
  isExpanded(key: string): boolean { return this.expanded().has(key); }
  toggleDetails(key: string): void {
    const next = new Set(this.expanded());
    if (next.has(key)) next.delete(key); else next.add(key);
    this.expanded.set(next);
  }
  statusLabel(status: string): string {
    switch (status) {
      case 'success': return 'Completed';
      case 'skipped': return 'Skipped';
      case 'failed': return 'Failed';
      case 'partial_failure': return 'Partial failure';
      default: return `Unknown outcome (${status || 'empty status'})`;
    }
  }
  statusIcon(status: string): string {
    switch (status) {
      case 'success': return '✓';
      case 'skipped': return '↷';
      case 'failed': return '!';
      case 'partial_failure': return '◐';
      default: return '?';
    }
  }
  isCollision(record: ActivityRecord): boolean {
    return record.status === 'skipped' && Boolean(record.error?.startsWith('Destination already exists'));
  }
  operationSummary(record: ActivityRecord): string {
    const actions = record.completedActions.length ? record.completedActions : record.intendedActions;
    return actions.length ? actions.join(' → ') : 'No file actions recorded';
  }
  pathSummary(record: ActivityRecord): string {
    if (!record.resultingPath) return '';
    const parts = record.resultingPath.split(/[\\/]/).filter(Boolean);
    return parts.slice(-2).join('/');
  }
  private eventKey(record: ActivityRecord): string {
    return [record.timestamp, record.ruleId, record.sourcePath, record.status].join('|');
  }
  private dateLabel(date: Date): string {
    const today = new Date();
    const dayStart = (value: Date) => new Date(value.getFullYear(), value.getMonth(), value.getDate()).getTime();
    const difference = (dayStart(today) - dayStart(date)) / 86_400_000;
    if (difference === 0) return 'Today';
    if (difference === 1) return 'Yesterday';
    return new Intl.DateTimeFormat(undefined, { dateStyle: 'long' }).format(date);
  }

}
