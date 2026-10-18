import { TestBed } from '@angular/core/testing';
import { provideRouter, Router } from '@angular/router';
import { vi } from 'vitest';
import { SavedRule } from '../../core/models/interpretation.model';
import { RuleStoreService } from '../../core/services/rule-store.service';
import { DashboardPagePage } from './dashboard-page.page';

describe('DashboardPagePage', () => {
  const firstRule: SavedRule = {
    id: 'rule-pdfs',
    title: 'Sort new PDFs',
    trigger: { type: 'file_created', extensions: ['.pdf'] },
    actions: [
      { type: 'rename', pattern: '{date}-{originalName}' },
      { type: 'move', destinationFolderId: 'folder-reports' },
    ],
    sourceFolderId: 'folder-inbox',
    enabled: true,
    createdAt: 1,
    updatedAt: 1,
  };

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [DashboardPagePage],
      providers: [provideRouter([])],
    }).compileComponents();

    const store = TestBed.inject(RuleStoreService);
    vi.spyOn(store, 'load').mockResolvedValue(undefined);
    vi.spyOn(store, 'refreshRuntimeStatuses').mockResolvedValue(undefined);
  });

  it('shows an empty workspace and a path to create a rule', () => {
    const fixture = TestBed.createComponent(DashboardPagePage);
    fixture.detectChanges();

    expect(fixture.nativeElement.textContent).toContain('Your workspace is ready');
    expect(fixture.nativeElement.querySelector('a[routerLink="/rules"]')).not.toBeNull();
    fixture.destroy();
  });

  it('shows rule details, ordered actions, authorized folders, and live errors', () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([firstRule]);
    store.folders.set([
      { id: 'folder-inbox', name: 'Inbox' },
      { id: 'folder-reports', name: 'Reports' },
    ]);
    store.runtimeStatuses.set([{ ruleId: firstRule.id, status: 'Error', error: 'Folder access was lost.' }]);

    const fixture = TestBed.createComponent(DashboardPagePage);
    fixture.detectChanges();
    (fixture.nativeElement.querySelector('.workbench-rule') as HTMLButtonElement).click();
    fixture.detectChanges();

    const page = fixture.nativeElement as HTMLElement;
    expect(page.querySelector('.inspector-title h2')?.textContent).toContain(firstRule.title);
    expect(page.textContent).toContain('Enabled');
    expect(page.textContent).toContain('Folder access was lost.');
    expect(page.textContent).toContain('Inbox');
    expect(page.textContent).toContain('Reports');
    expect(page.textContent).toContain('When a new file is created');
    expect(page.textContent).toContain('Rename to {date}-{originalName}');
    expect(page.textContent).toContain('Move to Reports');
    expect(page.textContent).toContain('Disable rule');
    expect(page.textContent).toContain('Dry Run');
    expect(page.textContent).toContain('Edit');
    expect(page.textContent).toContain('Delete');
    expect(page.textContent).not.toContain('folder-inbox');
    fixture.destroy();
  });

  it('updates selection with arrow keys and focuses the selected row', () => {
    const store = TestBed.inject(RuleStoreService);
    const secondRule = { ...firstRule, id: 'rule-images', title: 'Rename images' };
    store.rules.set([firstRule, secondRule]);

    const fixture = TestBed.createComponent(DashboardPagePage);
    fixture.detectChanges();
    const rows = [...(fixture.nativeElement as HTMLElement).querySelectorAll<HTMLButtonElement>('.workbench-rule')];
    rows[0].focus();
    rows[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    fixture.detectChanges();

    expect(fixture.componentInstance.selectedRuleId()).toBe(secondRule.id);
    expect(document.activeElement).toBe(rows[1]);
    fixture.destroy();
  });

  it('keeps enable confirmation and delegates the approved action to the store', async () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([{ ...firstRule, enabled: false }]);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    const setEnabled = vi.spyOn(store, 'setEnabled').mockResolvedValue(undefined);
    const fixture = TestBed.createComponent(DashboardPagePage);
    fixture.detectChanges();
    fixture.componentInstance.selectRule(store.rules()[0]);
    fixture.detectChanges();
    const enable = [...(fixture.nativeElement as HTMLElement).querySelectorAll('button')]
      .find((button) => button.textContent?.trim() === 'Enable rule')!;
    enable.click();
    await fixture.whenStable();

    expect(window.confirm).toHaveBeenCalled();
    expect(setEnabled).toHaveBeenCalledWith(firstRule.id, true);
    fixture.destroy();
  });

  it('opens the Dry Run picker without previewing when the picker is cancelled', async () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([firstRule]);
    const chooseFile = vi.spyOn(store, 'choosePreviewFile').mockResolvedValue(null);
    const preview = vi.spyOn(store, 'preview');
    const fixture = TestBed.createComponent(DashboardPagePage);
    fixture.detectChanges();
    fixture.componentInstance.selectRule(firstRule);
    fixture.detectChanges();
    [...(fixture.nativeElement as HTMLElement).querySelectorAll('button')]
      .find((button) => button.textContent?.trim() === 'Dry Run')!.click();
    await fixture.whenStable();

    expect(chooseFile).toHaveBeenCalledWith(firstRule.id);
    expect(preview).not.toHaveBeenCalled();
    expect(fixture.componentInstance.dryRunLoading()).toBe(false);
    fixture.destroy();
  });

  it('confirms deletion and clears the selected rule after it succeeds', async () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([firstRule]);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    const remove = vi.spyOn(store, 'delete').mockImplementation(async () => {
      store.rules.set([]);
    });
    const fixture = TestBed.createComponent(DashboardPagePage);
    fixture.detectChanges();
    fixture.componentInstance.selectRule(firstRule);
    await fixture.componentInstance.delete(firstRule);
    fixture.detectChanges();

    expect(window.confirm).toHaveBeenCalled();
    expect(remove).toHaveBeenCalledWith(firstRule.id);
    expect(fixture.componentInstance.selectedRule()).toBeNull();
    expect(fixture.nativeElement.textContent).toContain('Your workspace is ready');
    fixture.destroy();
  });

  it('opens the create route with the selected rule id when Edit is used', () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([firstRule]);
    const router = TestBed.inject(Router);
    const navigate = vi.spyOn(router, 'navigate').mockResolvedValue(true);
    const fixture = TestBed.createComponent(DashboardPagePage);

    fixture.componentInstance.edit(firstRule);

    expect(navigate).toHaveBeenCalledWith(['/rules'], { state: { editRuleId: firstRule.id } });
    fixture.destroy();
  });
});
