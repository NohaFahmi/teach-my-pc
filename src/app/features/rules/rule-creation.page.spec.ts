import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { vi } from 'vitest';
import { SavedRule } from '../../core/models/interpretation.model';
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
