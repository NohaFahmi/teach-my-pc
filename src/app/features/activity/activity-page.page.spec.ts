import { TestBed } from '@angular/core/testing';
import { vi } from 'vitest';
import { RuleStoreService } from '../../core/services/rule-store.service';
import { ActivityRecord } from '../../core/models/interpretation.model';
import { ActivityPagePage } from './activity-page.page';

const event = (overrides: Partial<ActivityRecord> = {}): ActivityRecord => ({
  ruleId: 'rule-1', timestamp: Math.floor(new Date(2026, 9, 18, 12).getTime() / 1000),
  sourcePath: '/Users/me/Inbox/invoice.pdf', sourceFilename: 'invoice.pdf',
  intendedActions: ['Move to Invoices'], completedActions: ['Move to Invoices'],
  resultingPath: '/Users/me/Invoices/invoice.pdf', status: 'success', error: null, ...overrides,
});

describe('ActivityPagePage', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [ActivityPagePage] }).compileComponents();
    const store = TestBed.inject(RuleStoreService);
    vi.spyOn(store, 'load').mockResolvedValue(undefined);
    vi.spyOn(store, 'loadActivity').mockResolvedValue(undefined);
  });

  function render(records: ActivityRecord[] = []) {
    TestBed.inject(RuleStoreService).activities.set(records);
    const fixture = TestBed.createComponent(ActivityPagePage);
    fixture.detectChanges();
    return fixture;
  }

  it('renders a successful event with timeline labels, local date and related rule', () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([{ id: 'rule-1', title: 'Organize PDFs', trigger: { type: 'file_created', extensions: ['.pdf'] }, actions: [{ type: 'move', destinationFolderId: 'folder-1' }], sourceFolderId: 'source-1', enabled: true, createdAt: 1, updatedAt: 1 }]);
    const fixture = render([event()]);
    const page = fixture.nativeElement as HTMLElement;
    expect(page.textContent).toContain('Completed');
    expect(page.textContent).toContain('Move to Invoices');
    expect(page.textContent).toContain('Organize PDFs');
    expect(page.textContent).toContain('Today');
    fixture.destroy();
  });

  it('maps outcome filters, keeps unknown outcomes visible under All, and provides a safe label', () => {
    const fixture = render([
      event({ status: 'success' }), event({ status: 'skipped', sourcePath: '/a/skip.pdf' }),
      event({ status: 'failed', sourcePath: '/a/fail.pdf' }),
      event({ status: 'partial_failure', sourcePath: '/a/partial.pdf' }),
      event({ status: 'future_status', sourcePath: '/a/future.pdf' }),
    ]);
    const page = fixture.nativeElement as HTMLElement;
    expect(page.textContent).toContain('Unknown outcome (future_status)');
    const buttons = [...page.querySelectorAll<HTMLButtonElement>('.activity-filters button')];
    for (const [index, expected] of [[1, 'Completed'], [2, 'Skipped'], [3, 'Failed'], [4, 'Partial failure']] as const) {
      buttons[index].click(); fixture.detectChanges();
      expect(page.textContent).toContain(expected);
      expect(page.querySelectorAll('.event').length).toBe(1);
    }
    buttons[0].click(); fixture.detectChanges();
    expect(page.querySelectorAll('.event').length).toBe(5);
    fixture.destroy();
  });

  it('shows full paths and exact diagnostics, preserving the completed actions on partial failure', () => {
    const fixture = render([event({ status: 'partial_failure', completedActions: ['Rename to invoice-final.pdf'], error: 'Move failed after an earlier action', resultingPath: '/Users/me/Inbox/invoice-final.pdf' })]);
    const page = fixture.nativeElement as HTMLElement;
    page.querySelector<HTMLButtonElement>('.details-toggle')!.click(); fixture.detectChanges();
    expect(page.textContent).toContain('/Users/me/Inbox/invoice.pdf');
    expect(page.textContent).toContain('Rename to invoice-final.pdf');
    expect(page.textContent).toContain('/Users/me/Inbox/invoice-final.pdf');
    expect(page.textContent).toContain('Move failed after an earlier action');
    expect(page.textContent).toContain('1 action completed before failure');
    fixture.destroy();
  });

  it('labels a collision skip as preserving the source only for the confirmed collision record', () => {
    const fixture = render([event({ status: 'skipped', completedActions: [], error: 'Destination already exists: /archive/invoice.pdf' })]);
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('The source file was preserved');
    fixture.destroy();
  });

  it('truncates long filenames visually while retaining the source path in expandable details', () => {
    const longName = `${'long-'.repeat(40)}file.pdf`;
    const fixture = render([event({ sourceFilename: longName, sourcePath: `/Users/me/Inbox/${longName}` })]);
    const page = fixture.nativeElement as HTMLElement;
    expect(page.querySelector('h3')?.textContent).toBe(longName);
    page.querySelector<HTMLButtonElement>('.details-toggle')!.click(); fixture.detectChanges();
    expect(page.textContent).toContain(`/Users/me/Inbox/${longName}`);
    fixture.destroy();
  });

  it('preserves expanded events when polling replaces the record array', () => {
    const store = TestBed.inject(RuleStoreService);
    store.activities.set([event()]);
    const fixture = TestBed.createComponent(ActivityPagePage); fixture.detectChanges();
    (fixture.nativeElement as HTMLElement).querySelector<HTMLButtonElement>('.details-toggle')!.click(); fixture.detectChanges();
    store.activities.set([{ ...event() }]); fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).querySelector('.event-details')).not.toBeNull();
    fixture.destroy();
  });

  it('renders loading, empty, no-match, and error states', () => {
    const store = TestBed.inject(RuleStoreService);
    store.activityLoading.set(true);
    let fixture = render();
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('Loading activity'); fixture.destroy();
    store.activityLoading.set(false);
    fixture = render(); expect((fixture.nativeElement as HTMLElement).textContent).toContain('No activity yet'); fixture.destroy();
    fixture = render([event()]);
    (fixture.nativeElement as HTMLElement).querySelectorAll<HTMLButtonElement>('.activity-filters button')[2].click(); fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('No matching activity'); fixture.destroy();
    store.activityError.set('storage unavailable');
    fixture = render([event()]); expect((fixture.nativeElement as HTMLElement).textContent).toContain('storage unavailable'); fixture.destroy();
    store.activityError.set('');
  });

  it('refreshes activity on the existing interval while the page is open', async () => {
    vi.useFakeTimers();
    const refresh = vi.mocked(TestBed.inject(RuleStoreService).loadActivity);
    const fixture = TestBed.createComponent(ActivityPagePage);
    await vi.advanceTimersByTimeAsync(1500);
    expect(refresh).toHaveBeenCalledTimes(2);
    fixture.destroy(); vi.useRealTimers();
  });
});
