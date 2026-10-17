import { TestBed } from '@angular/core/testing';
import { vi } from 'vitest';
import { RuleCreationPage } from './rule-creation.page';
import { RuleStoreService } from '../../core/services/rule-store.service';
import { RuleInterpreterService } from '../../core/services/rule-interpreter.service';
import { SavedRule } from '../../core/models/interpretation.model';

describe('RuleCreationPage', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [RuleCreationPage] }).compileComponents();
    vi.spyOn(TestBed.inject(RuleStoreService), 'load').mockResolvedValue(undefined);
    vi.spyOn(window, 'scrollTo').mockImplementation(() => undefined);
  });

  const savedRule: SavedRule = {
    id: 'internal-rule-id', title: 'Organize PDFs in Documents',
    trigger: { type: 'file_created', extensions: ['.pdf'] },
    actions: [{ type: 'rename', pattern: '{date}-{originalName}' }, { type: 'move', destinationFolderId: 'folder-id' }],
    sourceFolderId: 'source-id', enabled: false, createdAt: 1, updatedAt: 1,
  };

  it('keeps the current source folder when the native picker is cancelled', async () => {
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

  it('stacks the library kicker above its heading and renders useful rule metadata', () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([savedRule]);
    const fixture = TestBed.createComponent(RuleCreationPage);
    fixture.detectChanges();
    const compiled = fixture.nativeElement as HTMLElement;
    const header = compiled.querySelector('.library-heading > div')!;
    expect(header.children[0].textContent?.trim()).toBe('YOUR LIBRARY');
    expect(header.children[1].textContent?.trim()).toBe('My Rules');
    expect(compiled.querySelector('.rule-count')?.textContent).toContain('1 saved');
    expect(compiled.querySelector('.saved-rule h3')?.textContent).toContain(savedRule.title);
    expect(compiled.querySelector('.saved-rule-meta')?.textContent).toContain('Rename → Move');
    expect(compiled.querySelector('.runtime-status')?.textContent).toContain('Disabled');
    expect(compiled.textContent).not.toContain(savedRule.id);
    fixture.destroy();
  });

  it('passes the interpreter title into the editable builder', async () => {
    const interpreter = TestBed.inject(RuleInterpreterService);
    vi.spyOn(interpreter, 'interpret').mockResolvedValue(undefined);
    interpreter.state.set({ status: 'result', result: {
      status: 'supported', rule: { title: savedRule.title, trigger: savedRule.trigger, actions: [{ type: 'move', destination: 'Documents', requiresFolderSelection: true }] },
      message: 'Move new PDF files to Documents', clarificationQuestions: [],
    } });
    const fixture = TestBed.createComponent(RuleCreationPage);
    const page = fixture.componentInstance;
    await page.generate();
    expect(page.title()).toBe('Organize PDFs in Documents');
    expect(page.titleManuallyEdited()).toBe(false);
    fixture.destroy();
  });

  it('keeps a user-edited title when saving', async () => {
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
    fixture.destroy();
  });

  it('does not mutate an existing saved rule while opening it for editing', () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([savedRule]);
    const fixture = TestBed.createComponent(RuleCreationPage);
    fixture.componentInstance.edit(savedRule);
    fixture.componentInstance.title.set('Unsaved edit');
    expect(store.rules()[0]).toEqual(savedRule);
    fixture.destroy();
  });

  it('renders an 80-character rule title without losing its text', () => {
    const longTitle = 'Detailed quarterly document filing rules for the operations department '.repeat(2).slice(0, 80).trimEnd().padEnd(80, 'x');
    const longRule = { ...savedRule, title: longTitle };
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([longRule]);
    const fixture = TestBed.createComponent(RuleCreationPage);
    fixture.detectChanges();
    const heading = fixture.nativeElement.querySelector('.saved-rule h3') as HTMLHeadingElement;
    expect(heading.textContent?.trim()).toBe(longRule.title);
    expect(heading.title).toBe(longRule.title);
    fixture.destroy();
  });

  it('opens the picker on Dry Run and leaves preview state unchanged when cancelled', async () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([savedRule]);
    const picker = vi.spyOn(store, 'choosePreviewFile').mockResolvedValue(null);
    const preview = vi.spyOn(store, 'preview');
    const fixture = TestBed.createComponent(RuleCreationPage);
    fixture.detectChanges();
    const button = [...(fixture.nativeElement as HTMLElement).querySelectorAll('button')].find((item) => item.textContent?.trim() === 'Dry run')!;
    button.click();
    expect(picker).toHaveBeenCalledWith(savedRule.id);
    await fixture.whenStable();
    fixture.detectChanges();
    expect(preview).not.toHaveBeenCalled();
    expect(fixture.nativeElement.textContent).not.toContain('Dry Run ·');
    expect(fixture.componentInstance.dryRunLoading()).toBe(false);
    fixture.destroy();
  });

  it('renders planner response and shows IPC errors in the preview panel', async () => {
    const store = TestBed.inject(RuleStoreService);
    store.rules.set([savedRule]);
    vi.spyOn(store, 'choosePreviewFile').mockResolvedValue('/authorized/test.pdf');
    vi.spyOn(store, 'preview').mockResolvedValue({ sourceFilename: 'test.pdf', matchesRule: true, message: null, willModifyFiles: false, actions: ['Rename to 2026-10-17-test.pdf', 'Move to /authorized/destination/2026-10-17-test.pdf'], errors: [], collisions: [] });
    const fixture = TestBed.createComponent(RuleCreationPage);
    await fixture.componentInstance.dryRun(savedRule);
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent).toContain(savedRule.title);
    expect(fixture.nativeElement.textContent).toContain('test.pdf');
    expect(fixture.nativeElement.textContent).toContain('No files will be modified');
    fixture.destroy();

    vi.spyOn(store, 'choosePreviewFile').mockRejectedValue(new Error('Picker unavailable'));
    const failed = TestBed.createComponent(RuleCreationPage);
    await failed.componentInstance.dryRun(savedRule);
    failed.detectChanges();
    expect(failed.nativeElement.textContent).toContain('Picker unavailable');
    expect(failed.componentInstance.dryRunLoading()).toBe(false);
    failed.destroy();
  });
});
