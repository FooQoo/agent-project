import type { TaskSummary, ColumnName } from '../../../shared/api/contracts';
import type { TFunction } from 'i18next';
import { pad } from '../../../shared/lib/time/formatTime';

/** Local time as `MM/DD HH:mm` from an RFC 3339 timestamp. */
export function formatShort(iso: string): string {
  const d = new Date(iso);
  return `${pad(d.getMonth() + 1)}/${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function taskCountLabel(n: number, t?: TFunction): string {
  return t ? t('status.cardCount', { count: n }) : n === 1 ? '1 task' : `${n} tasks`;
}

const COLUMN_LABELS: Record<ColumnName, string> = {
  backlog: 'Backlog',
  todo: 'Todo',
  doing: 'Doing',
  done: 'Done',
};

export function columnLabel(c: ColumnName, t?: TFunction): string {
  return t ? t(`column.${c}`) : COLUMN_LABELS[c];
}

const STATE_LABELS = {
  idle: 'Idle',
  running: 'Running',
  paused: 'Paused',
} as const;

const AWAITING_ANSWER_LABEL = 'Awaiting answer';

/** The run state is shown for cards in Doing; a card that waits for an answer shows that instead. */
export function stateLabel(task: TaskSummary, t?: TFunction): string | null {
  // An AI card shows its run state in Doing; elsewhere only a question that waits for an answer
  // is worth a label, and that label takes precedence over the run state.
  if (task.assignee !== 'ai') return null;
  if (task.awaitingAnswer) return t ? t('runState.awaitingAnswer') : AWAITING_ANSWER_LABEL;
  if (task.taskStatus !== 'doing') return null;
  return t ? t(`runState.${task.agentStatus}`) : STATE_LABELS[task.agentStatus];
}
