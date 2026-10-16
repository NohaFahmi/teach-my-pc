import { Component, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RuleAction } from '../../core/models/interpretation.model';
import { RuleInterpreterService } from '../../core/services/rule-interpreter.service';

@Component({ selector: 'app-rule-creation-page', imports: [FormsModule], templateUrl: './rule-creation.page.html', styleUrl: './rule-creation.page.scss' })
export class RuleCreationPage {
  readonly interpreter = inject(RuleInterpreterService);
  readonly instruction = signal('');
  readonly canGenerate = computed(() => this.instruction().trim().length > 0 && this.interpreter.state().status !== 'loading');
  readonly examples = [
    'Rename new PDFs with today’s date and their original name.',
    'Move new JPG files to my Photos folder.',
    'لما ينزل ملف PDF جديد، انسخه إلى مجلد الفواتير',
  ];
  async generate(): Promise<void> { await this.interpreter.interpret(this.instruction()); }
  useExample(example: string): void { this.instruction.set(example); }
  actionLabel(action: RuleAction): string {
    switch (action.type) {
      case 'rename': return 'Rename to ' + action.pattern + ' (keep original extension)';
      case 'move': return 'Move to ' + action.destination;
      case 'copy': return 'Copy to ' + action.destination;
    }
  }
}
