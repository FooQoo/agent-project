import { useTranslation } from 'react-i18next';
import { CheckIcon, ClockIcon, LoaderIcon, PauseIcon } from 'lucide-react';
import { Badge } from '@/shared/ui/badge';
import type { ColumnName, RunState } from '../../../shared/api/contracts';

export function TaskStatusBadge({ status }: { status: ColumnName }) {
  const { t } = useTranslation();
  return (
    <Badge variant={status === 'doing' ? 'default' : status === 'done' ? 'secondary' : 'outline'}>
      {status === 'done' && <CheckIcon aria-hidden="true" className="text-[color:var(--r-done)]" />}
      {t(`column.${status}`)}
    </Badge>
  );
}

export function RunStatusBadge({ status }: { status: RunState }) {
  const { t } = useTranslation();
  return (
    <Badge
      variant={
        status === 'awaiting_answer'
          ? 'pending'
          : status === 'running'
            ? 'secondary'
            : status === 'paused'
              ? 'outline'
              : 'muted'
      }
    >
      {status === 'running' && (
        <LoaderIcon aria-hidden="true" className="text-[color:var(--r-text2)]" />
      )}
      {status === 'awaiting_answer' && <PauseIcon aria-hidden="true" />}
      {status === 'paused' && <ClockIcon aria-hidden="true" />}
      {status === 'ran' && <CheckIcon aria-hidden="true" />}
      {t(`runState.${status}`)}
    </Badge>
  );
}
