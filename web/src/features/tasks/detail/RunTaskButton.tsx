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
  if (
    !projectId ||
    task.assignee !== 'ai' ||
    (task.taskStatus !== 'todo' && task.taskStatus !== 'doing')
  ) {
    return null;
  }

  const runNow = async () => {
    setState('sending');
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
      setState(response.ok ? 'queued' : 'error');
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
          ? '実行キュー済み'
          : 'Queued'
        : state === 'error'
          ? ja
            ? '再試行'
            : 'Retry'
          : ja
            ? '今すぐ実行'
            : 'Run now';

  return (
    <Button
      type="button"
      size="s"
      className="detail-btn"
      disabled={disabled || state === 'sending'}
      onClick={() => void runNow()}
    >
      <PlayIcon aria-hidden="true" />
      {label}
    </Button>
  );
}
