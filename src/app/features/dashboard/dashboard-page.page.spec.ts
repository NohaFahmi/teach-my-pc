import { TestBed } from '@angular/core/testing';
import { vi } from 'vitest';
import { provideRouter } from '@angular/router';
import { RuleStoreService } from '../../core/services/rule-store.service';
import { DashboardPagePage } from './dashboard-page.page';

describe('DashboardPagePage', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [DashboardPagePage], providers: [provideRouter([])] }).compileComponents();
    const store = TestBed.inject(RuleStoreService);
    vi.spyOn(store, 'load').mockResolvedValue(undefined);
    vi.spyOn(store, 'loadActivity').mockResolvedValue(undefined);
    vi.spyOn(store, 'refreshRuntimeStatuses').mockResolvedValue(undefined);
  });

  it('shows counts and recent persisted activity from the store', () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([
      { id: 'r1', title: 'Sort PDFs', trigger: { type: 'file_created', extensions: ['.pdf'] }, actions: [{ type: 'move', destinationFolderId: 'f' }], sourceFolderId: 's', enabled: true, createdAt: 1, updatedAt: 1 },
      { id: 'r2', title: 'Rename images', trigger: { type: 'file_created', extensions: ['.png'] }, actions: [{ type: 'rename', pattern: '{originalName}' }], sourceFolderId: 's', enabled: false, createdAt: 1, updatedAt: 1 },
    ]);
    store.runtimeStatuses.set([{ ruleId: 'r1', status: 'Running', error: null }]);
    store.activities.set([{ ruleId: 'r1', timestamp: 1_800_000_000, sourcePath: '/hidden/report.pdf', sourceFilename: 'report.pdf', intendedActions: ['Move'], completedActions: ['Moved to Reports'], resultingPath: null, status: 'success', error: null }]);
    const fixture = TestBed.createComponent(DashboardPagePage);
    fixture.detectChanges();
    const page = fixture.nativeElement as HTMLElement;
    expect(page.textContent).toContain('Saved rules');
    expect(page.textContent).toContain('Running now');
    expect(page.textContent).toContain('Moved to Reports');
    expect(page.textContent).not.toContain('/hidden/report.pdf');
    expect(page.querySelector('a.primary-action')?.textContent).toContain('Create Rule');
    fixture.destroy();
  });
});
