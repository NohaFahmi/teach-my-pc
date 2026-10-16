export type InterpretationStatus = 'supported' | 'needs_clarification' | 'unsupported';
export interface ProposedRule { trigger: RuleTrigger; actions: RuleAction[]; }
export interface RuleTrigger { type: 'file_created'; extensions: string[]; }
export type RuleAction =
  | { type: 'rename'; pattern: string }
  | { type: 'move' | 'copy'; destination: string; requiresFolderSelection: true };
export interface InterpretationResult { status: InterpretationStatus; rule: ProposedRule | null; message: string; clarificationQuestions: string[]; }
export type InterpretationErrorCode = 'empty_instruction' | 'instruction_too_long' | 'ollama_unavailable' | 'model_missing' | 'timeout' | 'malformed_output' | 'schema_validation' | 'internal';
export interface InterpretationError { code: InterpretationErrorCode; field?: string; reason?: string; }
