export type InterpretationStatus = 'supported' | 'needs_clarification' | 'unsupported';
export interface ProposedRule {
  title?: string;
  trigger: RuleTrigger;
  actions: RuleAction[];
}
export interface RuleTrigger {
  type: 'file_created';
  extensions: string[];
}
export type RuleAction =
  | { type: 'rename'; pattern: string }
  | { type: 'move' | 'copy'; destination: string; requiresFolderSelection: true };
export interface InterpretationResult {
  status: InterpretationStatus;
  rule: ProposedRule | null;
  message: string;
  clarificationQuestions: string[];
}
export interface SavedRule {
  id: string;
  title: string;
  trigger: RuleTrigger;
  actions: SavedAction[];
  sourceFolderId: string;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
}
export type SavedAction =
  { type: 'rename'; pattern: string } | { type: 'move' | 'copy'; destinationFolderId: string };
export interface FolderChoice {
  id: string;
  name: string;
}
export interface ActivityRecord {
  ruleId: string;
  timestamp: number;
  sourcePath: string;
  sourceFilename: string;
  intendedActions: string[];
  completedActions: string[];
  resultingPath: string | null;
  status: 'success' | 'skipped' | 'failed';
  error: string | null;
}
export interface PlanPreview {
  sourceFilename: string;
  matchesRule: boolean;
  message: string | null;
  willModifyFiles: boolean;
  actions: string[];
  errors: string[];
  collisions: string[];
}
export type RuleRuntimeState = 'Starting' | 'Running' | 'Stopped' | 'Error';
export interface RuleRuntimeStatus {
  ruleId: string;
  status: RuleRuntimeState;
  error: string | null;
}
export type InterpretationErrorCode =
  | 'empty_instruction'
  | 'instruction_too_long'
  | 'ollama_unavailable'
  | 'model_missing'
  | 'timeout'
  | 'malformed_output'
  | 'schema_validation'
  | 'internal';
export interface InterpretationError {
  code: InterpretationErrorCode;
  field?: string;
  reason?: string;
}
