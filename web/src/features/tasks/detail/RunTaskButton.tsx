import { PlayIcon } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import type { TaskSummary } from '../../../shared/api/contracts';
import { authHeaders } from '../../../shared/api/http';

export function RunTaskButton({
  projectId,
  task,
  disabled,
}: {
  projectId: string | null;
  task: TaskSummary;
  disabled?: boolean;
}) {
  const { i18n } = useTranslation();
  const ja = i18n.language.startsWith('ja');
  const [state, setState] = useState<'idle' | 'sending' | 'queued' | 'error'>('idle');
  const [reason, setReason] = useState<string | null>(null);
  if (
    !projectId ||
    task.assignee !== 'ai' ||
    (task.taskStatus !== 'todo' && task.taskStatus !== 'doing')
  ) {
    return null;
  }

  const runNow = async () => {
    setState('sending');
    setReason(null);
    try {
      const response = await fetch(
        `/api/projects/${encodeURIComponent(projectId)}/tasks/${task.id}/run`,
        {
          method: 'POST',
          cache: 'no-store',
          headers: {
            Accept: 'application/json',
            'Content-Type': 'application/json',
            ...authHeaders(),
          },
          body: '{}',
        },
      );
      if (response.ok) {
        setState('queued');
      } else {
        const body = (await response.json().catch(() => null)) as { error?: string } | null;
        setReason(body?.error ?? null);
        setState('error');
      }
    } catch {
      setState('error');
    }
  };

  const label =
    state === 'sending'
      ? ja
        ? '起動中…'
        : 'Starting…'
      : state === 'queued'
        ? ja
          ? '実行開始'
          : 'Started'
        : state === 'error'
          ? (reason ?? (ja ? '起動不可' : 'Blocked'))
          : ja
            ? '今すぐ実行'
            : 'Run now';

  return (
    <Button
      type="button"
      size="s"
      className="detail-btn"
      title={reason ?? undefined}
      disabled={disabled || state === 'sending'}
      onClick={() => void runNow()}
    >
      <PlayIcon aria-hidden="true" />
      {label}
    </Button>
  );
}
