export type RapWorkKind = 'epic' | 'task';

export type RapBoardLane = 'backlog' | 'todo' | 'doing' | 'done' | 'canceled';

export type RapWorkPhase =
  | 'requested'
  | 'planning'
  | 'awaiting_approval'
  | 'ready'
  | 'executing'
  | 'revising'
  | 'verifying'
  | 'completed'
  | 'canceled';

export interface RapWorkItem {
  id: number;
  project_id: string;
  workspace: string;
  kind: RapWorkKind;
  parent_epic_id: number | null;
  title: string;
  rank: number;
  lane: RapBoardLane;
  phase: RapWorkPhase;
  version: number;
  current_revision_id: number | null;
  current_contract_id: number | null;
}

export interface RapChangeRequest {
  id: number;
  work_id: number;
  body: string;
  state: 'pending' | 'absorbed' | 'replanned' | 'rejected';
  received_at: number;
  resolution_reason: string | null;
  resolved_revision_id: number | null;
}

export interface RapResultLink {
  label: string;
  url: string;
}

export interface RapResult {
  id: number;
  work_id: number;
  contract_id: number;
  revision_id: number | null;
  stage: 'interim' | 'final';
  title: string;
  summary: string;
  links: RapResultLink[];
  document_hash: string | null;
  media_type: string | null;
  created_at: number;
}

export interface RapEvidence {
  id: number;
  work_id: number;
  contract_id: number;
  kind: 'verification' | 'final_result';
  summary: string;
  document_hash: string | null;
  recorded_at: number;
}

export interface RapCompletionCheck {
  ready: boolean;
  blockers: unknown[];
}

export interface RapSummaryItem {
  work: RapWorkItem;
  waiting_reason: string | null;
  retry_at: number | null;
  pending_change_requests: RapChangeRequest[];
  results: RapResult[];
  completion: RapCompletionCheck;
}

export interface RapSummary {
  sleeping: boolean;
  settings: { auto_run: boolean; max_sessions: number };
  items: RapSummaryItem[];
}

export interface RapWorkResults {
  results: RapResult[];
  evidence: RapEvidence[];
}

export interface RapWakeupReport {
  sleep_started_at: number | null;
  sleep_ended_at: number;
  delegated_decisions: unknown[];
  unanswered_questions: unknown[];
  recorded_evidence: RapEvidence[];
}

export interface RapEvent {
  kind: string;
  work_id: number | null;
  version: number | null;
}
