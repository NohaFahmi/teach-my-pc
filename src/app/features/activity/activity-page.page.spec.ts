import { TestBed } from '@angular/core/testing';
import { vi } from 'vitest';
import { RuleStoreService } from '../../core/services/rule-store.service';
import { ActivityPagePage } from './activity-page.page';

describe('ActivityPagePage', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [ActivityPagePage] }).compileComponents();
    const store = TestBed.inject(RuleStoreService);
    vi.spyOn(store, 'load').mockResolvedValue(undefined);
    vi.spyOn(store, 'loadActivity').mockResolvedValue(undefined);
  });

  it('loads persisted activity and renders rule, file, action, status, and error', () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([{ id: 'rule-1', title: 'Organize PDFs', trigger: { type: 'file_created', extensions: ['.pdf'] }, actions: [{ type: 'move', destinationFolderId: 'folder-1' }], sourceFolderId: 'source-1', enabled: true, createdAt: 1, updatedAt: 1 }]);
    store.activities.set([{ ruleId: 'rule-1', timestamp: 1_800_000_000, sourcePath: '/private/invoice.pdf', sourceFilename: 'invoice.pdf', intendedActions: ['Move'], completedActions: ['Move to Invoices'], resultingPath: '/Invoices/invoice.pdf', status: 'failed', error: 'Destination is unavailable' }]);
    const fixture = TestBed.createComponent(ActivityPagePage);
    fixture.detectChanges();
    const page = fixture.nativeElement as HTMLElement;
    expect(page.textContent).toContain('Organize PDFs');
    expect(page.textContent).toContain('invoice.pdf');
    expect(page.textContent).toContain('Move to Invoices');
    expect(page.textContent).toContain('failed');
    expect(page.textContent).toContain('Destination is unavailable');
    expect(page.textContent).not.toContain('/private/invoice.pdf');
    expect(TestBed.inject(RuleStoreService).loadActivity).toHaveBeenCalled();
    fixture.destroy();
  });

  it('refreshes activity while the page is open', async () => {
    vi.useFakeTimers();
    const store = TestBed.inject(RuleStoreService);
    const refresh = vi.mocked(store.loadActivity);
    const fixture = TestBed.createComponent(ActivityPagePage);
    await vi.advanceTimersByTimeAsync(1500);
    expect(refresh).toHaveBeenCalledTimes(2);
    fixture.destroy();
    vi.useRealTimers();
  });
});
