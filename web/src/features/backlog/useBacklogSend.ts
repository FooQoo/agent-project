import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ProjectId, TaskSummary } from '../../shared/api/contracts';
import { useWrite } from '../../shared/data/react/use-write';
import {
  type BacklogTarget,
  type SendOutcome,
  sendEpicTasks,
  sendTasks,
  withOpenQuestions,
} from './send';

const KEY = ['backlog-send'] as const;

export interface BacklogSend {
  /** The cards waiting for the person to confirm the warning, or `null`. */
  confirming: {
    ids: number[];
    warned: TaskSummary[];
    epicId?: string;
    /** The lane the cards go to. */
    to: BacklogTarget;
  } | null;
  busy: boolean;
  /** The failure of the last send, as a sentence for a notice, or `null`. A send that went through has none. */
  notice: string | null;
  dismissNotice: () => void;
  /** Sends the cards, or asks for the confirmation first when one of them has an open question. */
  request: (ids: readonly number[], epicId?: string, to?: BacklogTarget) => boolean;
  confirm: () => void;
  cancel: () => void;
}

/**
 * The send of Backlog cards, shared by the PC board and the phone: to Todo, or on the PC board to
 * Doing or Done. Cards with an open question are confirmed first (the warning names them and the
 * lane); only a failure or a partial send shows a notice, because the cards moving is the answer.
 */
export function useBacklogSend(
  projectId: ProjectId | null,
  tasks: readonly TaskSummary[],
  onSent: () => void,
): BacklogSend {
  const { t } = useTranslation();
  // A pending confirmation belongs to the project that asked for it: after a project change it is
  // neither shown nor confirmed.
  const [pending, setPending] = useState<{
    projectId: ProjectId | null;
    value: NonNullable<BacklogSend['confirming']>;
  } | null>(null);
  const confirming = pending !== null && pending.projectId === projectId ? pending.value : null;
  const setConfirming = useCallback(
    (value: BacklogSend['confirming']) => setPending(value && { projectId, value }),
    [projectId],
  );
  const [notice, setNotice] = useState<string | null>(null);
  const dismissNotice = useCallback(() => setNotice(null), []);

  interface Send {
    ids: readonly number[];
    ack: boolean;
    epicId?: string;
    to: BacklogTarget;
  }
  const send = useWrite<SendOutcome, Send>({
    mutationKey: KEY,
    mutationFn: ({ ids, ack, epicId, to }) =>
      projectId === null
        ? Promise.resolve({ sent: [], failed: [] })
        : epicId
          ? sendEpicTasks(projectId, epicId, ids, ack)
          : sendTasks(projectId, ids, ack, to),
    onSuccess: ({ sent, failed }, { ids, to }) => {
      const lane = t(`column.${to}`);
      const k = to === 'todo' ? 'backlog' : 'backlog.move';
      if (sent.length === 0) setNotice(t(`${k}.failed`, { lane }));
      else if (failed.length > 0)
        setNotice(
          t(`${k}.partial`, { sent: sent.length, total: ids.length, failed: failed.length }),
        );
      // A send that went through shows no notice: the cards moving is the answer.
    },
    onSettled: () => {
      setConfirming(null);
      onSent();
    },
  });
  const { mutateAsync, isWriting } = send;
  const run = useCallback(
    async (ids: readonly number[], ack: boolean, epicId?: string, to: BacklogTarget = 'todo') => {
      if (projectId === null || isWriting() || ids.length === 0) return;
      await mutateAsync({ ids, ack, epicId, to }).catch(() => {});
    },
    [projectId, isWriting, mutateAsync],
  );

  const request = useCallback(
    (ids: readonly number[], epicId?: string, to: BacklogTarget = 'todo') => {
      if (ids.length === 0 || isWriting()) return false;
      const warned = withOpenQuestions(tasks, ids);
      if (warned.length > 0) {
        setConfirming({ ids: [...ids], warned, epicId, to });
        return false;
      }
      void run(ids, false, epicId, to);
      return true;
    },
    [tasks, run, setConfirming, isWriting],
  );
  const confirm = useCallback(() => {
    if (confirming) void run(confirming.ids, true, confirming.epicId, confirming.to);
  }, [confirming, run]);
  const cancel = useCallback(() => setConfirming(null), [setConfirming]);
  return { confirming, busy: send.isPending, notice, dismissNotice, request, confirm, cancel };
}
