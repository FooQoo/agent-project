import { useTranslation } from 'react-i18next';
import { CheckIcon, ClockIcon, LoaderIcon, PauseIcon } from 'lucide-react';
import { Badge } from '@/shared/ui/badge';
import type { ColumnName, RunState } from '../../../shared/api/contracts';

type Variant = 'default' | 'secondary' | 'outline' | 'muted' | 'pending';

/** Card status and agent run state share one badge; the icons are decoration next to the text. */
export function StatusLabel({
  kind,
  status,
  monitor = false,
  awaitingAnswer = false,
}: {
  kind: 'card' | 'agent';
  status: ColumnName | RunState;
  monitor?: boolean;
  /** The task waits for an answer; the label then replaces the run state. */
  awaitingAnswer?: boolean;
}) {
  const { t } = useTranslation();
  const waiting = kind === 'agent' && awaitingAnswer;
  const monitorState = kind === 'agent' && (status !== 'idle' || waiting);
  const monitorLane = kind === 'card' && status === 'done';
  const text =
    monitor && monitorState
      ? t(waiting ? 'monitor.status.awaitingAnswer' : `monitor.status.${status}`)
      : monitor && monitorLane
        ? t('monitor.status.done')
        : kind === 'card'
          ? t(`column.${status}`)
          : t(waiting ? 'runState.awaitingAnswer' : `runState.${status}`);
  let variant: Variant;
  let icon = null;
  if (kind === 'card') {
    variant = status === 'doing' ? 'default' : status === 'done' ? 'secondary' : 'outline';
    if (status === 'done') icon = <CheckIcon aria-hidden="true" className="text-[var(--r-done)]" />;
  } else if (waiting) {
    variant = 'pending';
    icon = <PauseIcon aria-hidden="true" />;
  } else if (status === 'running') {
    variant = 'secondary';
    icon = <LoaderIcon aria-hidden="true" className="spinner text-[var(--r-text2)]" />;
  } else if (status === 'paused') {
    variant = 'outline';
    icon = <ClockIcon aria-hidden="true" />;
  } else {
    variant = 'muted';
  }
  return (
    <Badge
      variant={variant}
      className={`status-label status-label--${kind} status-label--${waiting ? 'awaiting-answer' : status}`}
    >
      {icon}
      {text}
    </Badge>
  );
}
