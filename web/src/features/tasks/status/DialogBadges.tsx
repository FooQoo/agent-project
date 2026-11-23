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

export function RunStatusBadge({
  status,
  awaitingAnswer = false,
}: {
  status: RunState;
  awaitingAnswer?: boolean;
}) {
  const { t } = useTranslation();
  return (
    <Badge
      variant={
        awaitingAnswer
          ? 'pending'
          : status === 'running'
            ? 'secondary'
            : status === 'paused'
              ? 'outline'
              : 'muted'
      }
    >
      {!awaitingAnswer && status === 'running' && (
        <LoaderIcon aria-hidden="true" className="text-[color:var(--r-text2)]" />
      )}
      {awaitingAnswer && <PauseIcon aria-hidden="true" />}
      {!awaitingAnswer && status === 'paused' && <ClockIcon aria-hidden="true" />}
      {t(awaitingAnswer ? 'runState.awaitingAnswer' : `runState.${status}`)}
    </Badge>
  );
}
