import { Injectable, signal } from '@angular/core';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { InterpretationError, InterpretationResult } from '../models/interpretation.model';

export type InterpreterState = { status: 'idle' } | { status: 'loading' } | { status: 'result'; result: InterpretationResult } | { status: 'error'; message: string };

@Injectable({ providedIn: 'root' })
export class RuleInterpreterService {
  readonly state = signal<InterpreterState>({ status: 'idle' });
  async interpret(instruction: string): Promise<void> {
    this.state.set({ status: 'loading' });
    if (!isTauri()) { this.state.set({ status: 'error', message: 'Rule interpretation is available in the TeachMyPC desktop app.' }); return; }
    try {
      const result = await invoke<InterpretationResult>('interpret_instruction', { instruction });
      this.state.set({ status: 'result', result });
    } catch (error: unknown) { this.state.set({ status: 'error', message: this.errorMessage(error) }); }
  }
  private errorMessage(error: unknown): string {
    const code = this.readErrorCode(error);
    switch (code) {
      case 'empty_instruction': return 'Enter an instruction to continue.';
      case 'instruction_too_long': return 'Keep your instruction under 2,000 characters.';
      case 'ollama_unavailable': return 'Ollama is not responding. Start Ollama and try again.';
      case 'model_missing': return 'The Gemma 3 4B model is not installed. Pull gemma3:4b in Ollama, then retry.';
      case 'timeout': return 'The local model took too long to respond. Try again.';
      case 'malformed_output': return 'The local model returned an unusable preview, even after one correction. Please retry or rephrase the instruction.';
      case 'schema_validation': return 'The preview did not match the file type or action you asked for, even after one correction. State the extension and exact action, then retry.';
      default: return 'TeachMyPC could not interpret that instruction. Please retry.';
    }
  }
  private readErrorCode(error: unknown): InterpretationError['code'] | null {
    if (typeof error !== 'object' || error === null || !('code' in error)) return null;
    const code = error.code;
    const knownCodes: InterpretationError['code'][] = [
      'empty_instruction', 'instruction_too_long', 'ollama_unavailable', 'model_missing',
      'timeout', 'malformed_output', 'schema_validation', 'internal',
    ];
    return typeof code === 'string' ? knownCodes.find((knownCode) => knownCode === code) ?? null : null;
  }
}
