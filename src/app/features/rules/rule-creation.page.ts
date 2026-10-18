import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import {
  FolderChoice,
  ProposedRule,
  SavedAction,
  SavedRule,
} from '../../core/models/interpretation.model';
import { RuleInterpreterService } from '../../core/services/rule-interpreter.service';
import { RuleStoreService } from '../../core/services/rule-store.service';

type DraftAction =
  | { type: 'rename'; pattern: string }
  | { type: 'move' | 'copy'; destinationFolderId: string; destinationLabel: string };
@Component({
  selector: 'app-rule-creation-page',
  imports: [FormsModule],
  templateUrl: './rule-creation.page.html',
  styleUrl: './rule-creation.page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RuleCreationPage {
  readonly interpreter = inject(RuleInterpreterService);
  readonly store = inject(RuleStoreService);
  private readonly router = inject(Router);
  readonly instruction = signal('');
  readonly title = signal('');
  readonly titleManuallyEdited = signal(false);
  readonly extensions = signal('');
  readonly sourceFolder = signal<FolderChoice | null>(null);
  readonly actions = signal<DraftAction[]>([]);
  readonly editingId = signal('');
  readonly builderVisible = signal(false);
  readonly generationCancelled = signal(false);
  readonly error = signal('');
  readonly sourceFolderError = signal('');
  readonly busy = signal(false);
  readonly canGenerate = computed(
    () => this.instruction().trim().length > 0 && this.interpreter.state().status !== 'loading',
  );
  readonly workflowSteps = [
    'Describe automation',
    'Interpret and review',
    'Authorize folders',
    'Save disabled',
    'Dry run, then enable',
  ];
  readonly currentStep = computed(() => (this.sourceFolder() ? 2 : this.builderVisible() ? 1 : 0));
  readonly examples = [
    'Rename new PDFs with today’s date and their original name.',
    'Move new JPG files to my Photos folder.',
    'لما ينزل ملف PDF جديد، انسخه إلى مجلد الفواتير',
  ];
  private generationVersion = 0;
  constructor() {
    const editRuleId = this.router.currentNavigation()?.extras.state?.['editRuleId'];
    void this.store.load().then(() => {
      if (typeof editRuleId !== 'string') return;
      const rule = this.store.rules().find((saved) => saved.id === editRuleId);
      if (rule) this.edit(rule);
    });
  }
  async generate(): Promise<void> {
    const generationVersion = ++this.generationVersion;
    this.clearDraftFields();
    await this.interpreter.interpret(this.instruction());
    if (generationVersion !== this.generationVersion) return;
    const state = this.interpreter.state();
    if (state.status === 'result' && state.result.status === 'supported' && state.result.rule) {
      const rule = state.result.rule;
      this.editingId.set('');
      if (!this.titleManuallyEdited())
        this.title.set(rule.title?.trim() || this.fallbackTitle(rule));
      this.extensions.set(rule.trigger.extensions.join(', '));
      this.builderVisible.set(true);
      this.sourceFolder.set(null);
      this.actions.set(
        rule.actions.map((action) =>
          action.type === 'rename'
            ? { ...action }
            : { type: action.type, destinationFolderId: '', destinationLabel: action.destination },
        ),
      );
      this.error.set('');
    }
  }
  cancelGeneration(): void {
    this.generationVersion++;
    this.interpreter.cancel();
    this.generationCancelled.set(true);
  }
  discardDraft(): void {
    if (this.busy()) return;
    this.generationVersion++;
    this.interpreter.cancel();
    this.instruction.set('');
    this.clearDraftFields();
  }
  private clearDraftFields(): void {
    this.title.set('');
    this.titleManuallyEdited.set(false);
    this.extensions.set('');
    this.sourceFolder.set(null);
    this.actions.set([]);
    this.editingId.set('');
    this.builderVisible.set(false);
    this.generationCancelled.set(false);
    this.error.set('');
    this.sourceFolderError.set('');
  }
  useExample(example: string): void {
    this.instruction.set(example);
  }
  setTitle(value: string): void {
    this.title.set(value);
    this.titleManuallyEdited.set(true);
  }
  async pickSource(): Promise<void> {
    try {
      const folder = await this.store.chooseFolder();
      if (folder) {
        this.sourceFolder.set(folder);
        this.sourceFolderError.set('');
      }
    } catch (error: unknown) {
      this.error.set(this.errorText(error));
    }
  }
  async pickDestination(index: number): Promise<void> {
    try {
      const folder = await this.store.chooseFolder();
      if (folder)
        this.actions.update((actions) =>
          actions.map((action, i) =>
            i === index && action.type !== 'rename'
              ? { ...action, destinationFolderId: folder.id, destinationLabel: folder.name }
              : action,
          ),
        );
    } catch (error: unknown) {
      this.error.set(this.errorText(error));
    }
  }
  addAction(type: 'rename' | 'move' | 'copy'): void {
    this.actions.update((actions) => [
      ...actions,
      type === 'rename'
        ? { type, pattern: '{originalName}' }
        : { type, destinationFolderId: '', destinationLabel: '' },
    ]);
  }
  changeActionType(index: number, type: 'rename' | 'move' | 'copy'): void {
    this.actions.update((items) =>
      items.map((action, i) =>
        i === index
          ? type === 'rename'
            ? { type, pattern: '{originalName}' }
            : { type, destinationFolderId: '', destinationLabel: '' }
          : action,
      ),
    );
  }
  removeAction(index: number): void {
    this.actions.update((actions) => actions.filter((_, i) => i !== index));
  }
  moveAction(index: number, direction: -1 | 1): void {
    this.actions.update((items) => {
      const next = [...items];
      const target = index + direction;
      if (target >= 0 && target < next.length)
        [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  }
  updateAction(index: number, patch: Partial<DraftAction>): void {
    this.actions.update((items) =>
      items.map((action, i) => (i === index ? ({ ...action, ...patch } as DraftAction) : action)),
    );
  }
  async save(): Promise<void> {
    this.error.set('');
    this.sourceFolderError.set('');
    const ext = this.extensions()
      .split(',')
      .map((value) => value.trim())
      .filter(Boolean);
    if (!this.title().trim()) {
      this.error.set('Enter a rule title.');
      return;
    }
    if (!this.sourceFolder()) {
      this.sourceFolderError.set('Choose an authorized source folder before saving.');
      return;
    }
    if (!this.actions().length) {
      this.error.set('Add at least one action.');
      return;
    }
    if (this.actions().some((action) => action.type !== 'rename' && !action.destinationFolderId)) {
      this.error.set('Choose a destination for each move or copy action.');
      return;
    }
    const savedActions: SavedAction[] = this.actions().map((action) =>
      action.type === 'rename'
        ? { type: 'rename', pattern: action.pattern }
        : { type: action.type, destinationFolderId: action.destinationFolderId },
    );
    const rule: SavedRule = {
      id: this.editingId(),
      title: this.title().trim(),
      trigger: { type: 'file_created', extensions: ext },
      actions: savedActions,
      sourceFolderId: this.sourceFolder()!.id,
      enabled: false,
      createdAt: 0,
      updatedAt: 0,
    };
    this.busy.set(true);
    try {
      await this.store.save(rule);
      this.generationVersion++;
      this.interpreter.cancel();
      this.instruction.set('');
      this.editingId.set('');
      this.title.set('');
      this.titleManuallyEdited.set(false);
      this.extensions.set('');
      this.sourceFolder.set(null);
      this.actions.set([]);
      this.builderVisible.set(false);
      this.generationCancelled.set(false);
      this.error.set('');
      this.sourceFolderError.set('');
    } catch (error: unknown) {
      this.error.set(this.errorText(error));
    } finally {
      this.busy.set(false);
    }
  }
  edit(rule: SavedRule): void {
    this.editingId.set(rule.id);
    this.title.set(rule.title);
    this.extensions.set(rule.trigger.extensions.join(', '));
    this.titleManuallyEdited.set(true);
    this.builderVisible.set(true);
    this.sourceFolder.set(
      this.store.folders().find((folder) => folder.id === rule.sourceFolderId) ?? {
        id: rule.sourceFolderId,
        name: 'Authorized folder',
      },
    );
    this.actions.set(
      rule.actions.map((action) =>
        action.type === 'rename'
          ? { ...action }
          : { ...action, destinationLabel: this.folderName(action.destinationFolderId) },
      ),
    );
    this.error.set('');
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }
  folderName(id: string): string {
    return this.store.folders().find((folder) => folder.id === id)?.name ?? 'Authorized folder';
  }
  private errorText(error: unknown): string {
    if (typeof error === 'string') return error;
    if (error instanceof Error) return error.message;
    if (
      typeof error === 'object' &&
      error !== null &&
      'message' in error &&
      typeof error.message === 'string'
    )
      return error.message;
    return 'Unexpected backend error';
  }
  private fallbackTitle(rule: ProposedRule): string {
    const extension = rule.trigger.extensions[0]?.replace(/^\./, '').toUpperCase() ?? '';
    const destination = rule.actions.find(
      (action) => action.type === 'move' || action.type === 'copy',
    );
    const arabic = this.instruction()
      .split('')
      .some((character) => /[\u0600-\u06ff]/.test(character));
    const action = rule.actions[0];
    if (arabic) {
      const kind = /^(PNG|JPG|JPEG)$/.test(extension) ? 'صور' : 'ملفات';
      const fileType = extension ? `${kind} ${extension}` : 'الملفات';
      if (action?.type === 'rename')
        return `إعادة تسمية ${fileType}${action.pattern.includes('{date}') ? ' بالتاريخ' : ''}`;
      if (action?.type === 'copy')
        return `نسخ ${fileType}${destination?.type === 'copy' ? ` إلى ${destination.destination}` : ''}`;
      return `تنظيم ${fileType}${destination?.type === 'move' ? ` في ${destination.destination}` : ''}`;
    }
    const plural = extension ? `${extension}s` : 'Files';
    const fileType = extension ? `${extension} Files` : 'New Files';
    if (action?.type === 'rename')
      return action.pattern.includes('{date}') ? `Date-Stamp ${fileType}` : `Rename ${fileType}`;
    if (action?.type === 'copy')
      return `Back Up ${plural}${destination?.type === 'copy' ? ` to ${destination.destination}` : ''}`;
    return `Organize ${plural}${destination?.type === 'move' ? ` in ${destination.destination}` : ''}`;
  }
}
