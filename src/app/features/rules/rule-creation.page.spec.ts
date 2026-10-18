import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { vi } from 'vitest';
import { InterpretationResult, SavedRule } from '../../core/models/interpretation.model';
import { RuleInterpreterService } from '../../core/services/rule-interpreter.service';
import { RuleStoreService } from '../../core/services/rule-store.service';
import { RuleCreationPage } from './rule-creation.page';

describe('RuleCreationPage', () => {
  const savedRule: SavedRule = {
    id: 'rule-id',
    title: 'Organize PDFs in Documents',
    trigger: { type: 'file_created', extensions: ['.pdf'] },
    actions: [{ type: 'rename', pattern: '{date}-{originalName}' }],
    sourceFolderId: 'source-id',
    enabled: false,
    createdAt: 1,
    updatedAt: 1,
  };

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [RuleCreationPage],
      providers: [provideRouter([])],
    }).compileComponents();
    vi.spyOn(TestBed.inject(RuleStoreService), 'load').mockResolvedValue(undefined);
    vi.spyOn(window, 'scrollTo').mockImplementation(() => undefined);
  });

  it('keeps the selected source folder when the native picker is cancelled', async () => {
    const store = TestBed.inject(RuleStoreService);
    vi.spyOn(store, 'chooseFolder').mockResolvedValue(null);
    const fixture = TestBed.createComponent(RuleCreationPage);
    const page = fixture.componentInstance;
    const authorized = { id: 'selected-folder', name: 'Invoices' };
    page.sourceFolder.set(authorized);

    await page.pickSource();

    expect(page.sourceFolder()).toEqual(authorized);
    fixture.destroy();
  });

  it('keeps the route focused on creating and reviewing a rule', () => {
    const fixture = TestBed.createComponent(RuleCreationPage);
    fixture.detectChanges();

    expect(fixture.nativeElement.textContent).toContain('What should happen?');
    expect(fixture.nativeElement.textContent).toContain('Generate preview');
    expect(fixture.nativeElement.textContent).not.toContain('My Rules');
    fixture.destroy();
  });

  it('gives action selectors and controls position-specific accessible names after reorder and removal', () => {
    const fixture = TestBed.createComponent(RuleCreationPage);
    const page = fixture.componentInstance;
    page.builderVisible.set(true);
    page.actions.set([
      { type: 'rename', pattern: '{originalName}' },
      { type: 'move', destinationFolderId: '', destinationLabel: '' },
    ]);
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('label[for="action-type-0"]')?.textContent).toContain('Action 1 type');
    expect(fixture.nativeElement.querySelector('label[for="action-type-1"]')?.textContent).toContain('Action 2 type');
    expect(fixture.nativeElement.querySelector('[aria-label="Remove action 2"]')).not.toBeNull();

    page.moveAction(1, -1);
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('label[for="action-type-0"]')?.textContent).toContain('Action 1 type');
    expect(page.actions()[0]?.type).toBe('move');

    page.removeAction(0);
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelectorAll('.action-editor select')).toHaveLength(1);
    expect(fixture.nativeElement.querySelector('label[for="action-type-0"]')?.textContent).toContain('Action 1 type');
    fixture.destroy();
  });

  it('sets automatic prompt direction and places missing-source validation beside its control without saving', async () => {
    const store = TestBed.inject(RuleStoreService);
    const save = vi.spyOn(store, 'save');
    const fixture = TestBed.createComponent(RuleCreationPage);
    const page = fixture.componentInstance;
    page.builderVisible.set(true);
    page.title.set('A rule');
    page.actions.set([{ type: 'rename', pattern: '{originalName}' }]);
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('#instruction')?.getAttribute('dir')).toBe('auto');
    expect(fixture.nativeElement.querySelector('#instruction')?.getAttribute('aria-label')).toBeNull();
    await page.save();
    fixture.detectChanges();

    const folderError = fixture.nativeElement.querySelector('#source-folder-error');
    expect(folderError?.textContent).toContain('Choose an authorized source folder');
    expect(fixture.nativeElement.querySelector('.folder-row')?.contains(folderError)).toBe(true);
    expect(fixture.nativeElement.querySelector('.folder-row button')?.getAttribute('aria-describedby')).toBe('source-folder-error');
    expect(save).not.toHaveBeenCalled();
    fixture.destroy();
  });

  it('passes the interpreter title into the editable builder', async () => {
    const interpreter = TestBed.inject(RuleInterpreterService);
    vi.spyOn(interpreter, 'interpret').mockResolvedValue(undefined);
    interpreter.state.set({
      status: 'result',
      result: {
        status: 'supported',
        rule: {
          title: savedRule.title,
          trigger: savedRule.trigger,
          actions: [{ type: 'move', destination: 'Documents', requiresFolderSelection: true }],
        },
        message: 'Move new PDF files to Documents',
        clarificationQuestions: [],
      },
    });
    const fixture = TestBed.createComponent(RuleCreationPage);

    await fixture.componentInstance.generate();

    expect(fixture.componentInstance.title()).toBe(savedRule.title);
    expect(fixture.componentInstance.titleManuallyEdited()).toBe(false);
    fixture.destroy();
  });

  it('replaces draft actions when the user generates another interpretation', async () => {
    const interpreter = TestBed.inject(RuleInterpreterService);
    const results: InterpretationResult[] = [
      {
        status: 'supported',
        rule: {
          title: 'Rename then move PDFs',
          trigger: { type: 'file_created', extensions: ['.pdf'] },
          actions: [
            { type: 'rename', pattern: '{date}-{originalName}' },
            { type: 'move', destination: 'Archive', requiresFolderSelection: true },
          ],
        },
        message: 'Rename and move PDFs',
        clarificationQuestions: [],
      },
      {
        status: 'supported',
        rule: {
          title: 'Copy PDFs to Backup',
          trigger: { type: 'file_created', extensions: ['.pdf'] },
          actions: [{ type: 'copy', destination: 'Backup', requiresFolderSelection: true }],
        },
        message: 'Copy PDFs to Backup',
        clarificationQuestions: [],
      },
    ];
    let resultIndex = 0;
    vi.spyOn(interpreter, 'interpret').mockImplementation(async () => {
      const result = results[resultIndex++];
      if (result) interpreter.state.set({ status: 'result', result });
    });
    const fixture = TestBed.createComponent(RuleCreationPage);
    const page = fixture.componentInstance;

    page.instruction.set('Rename then move new PDFs');
    await page.generate();
    expect(page.actions()).toHaveLength(2);

    page.instruction.set('Copy new PDFs to Backup');
    await page.generate();

    expect(page.actions()).toEqual([
      { type: 'copy', destinationFolderId: '', destinationLabel: 'Backup' },
    ]);
    fixture.destroy();
  });

  it('keeps a user-edited title when saving a disabled rule', async () => {
    const store = TestBed.inject(RuleStoreService);
    const save = vi.spyOn(store, 'save').mockResolvedValue(savedRule);
    const fixture = TestBed.createComponent(RuleCreationPage);
    const page = fixture.componentInstance;
    page.title.set('Draft title');
    page.setTitle('My edited title');
    page.sourceFolder.set({ id: 'source-id', name: 'Inbox' });
    page.actions.set([{ type: 'rename', pattern: '{originalName}' }]);

    await page.save();

    expect(save.mock.calls[0][0].title).toBe('My edited title');
    expect(save.mock.calls[0][0].enabled).toBe(false);
    fixture.destroy();
  });

  it('does not mutate a saved rule while opening it for editing', () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([savedRule]);
    const fixture = TestBed.createComponent(RuleCreationPage);

    fixture.componentInstance.edit(savedRule);
    fixture.componentInstance.title.set('Unsaved edit');

    expect(store.rules()[0]).toEqual(savedRule);
    fixture.destroy();
  });
});
