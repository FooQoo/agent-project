// The writes of the board that are not a card's move: the order of a lane and the dialogs' save.
// Each is one mutation; what follows a write is the immediate read of the project.
import { useCallback } from 'react';
import { sendMessageRequest } from '../features/messages/ask/sendRequest';
import type { ActionDialogData } from '../features/tasks/model/actionData';
import type { ProjectId } from '../shared/api/contracts';
import { useWrite } from '../shared/data/react/use-write';
import { editTask, editTaskMultipart, reorderTask } from './api';
import type { AppControllerModel } from './contracts';

/**
 * The order of a lane, saved in the background: a refusal is not shown, and the board is read again
 * either way, so it shows the order the server holds.
 */
export function useReorder(refreshProject: (projectId: ProjectId) => void) {
  return useWrite<unknown, { projectId: ProjectId; id: number; beforeId: number | null }>({
    mutationKey: ['reorder'],
    mutationFn: ({ projectId, id, beforeId }) => reorderTask(projectId, id, beforeId),
    onSettled: (_result, _error, { projectId }) => refreshProject(projectId),
  });
}

type Session = AppControllerModel['session'];

/**
 * The save of the request and edit dialogs. A refusal rejects to the dialog, which shows the
 * server's reason; a save that went through reads the project again.
 */
export function useDialogSubmit(
  session: Session,
  refreshProject: (projectId: ProjectId) => void,
  openMessages: (target: { tab: 'fromYou' }) => void,
) {
  const { mutateAsync } = useWrite<
    void,
    { dialog: NonNullable<Session['dialog']>; projectId: ProjectId; data: ActionDialogData }
  >({
    mutationKey: ['dialog-submit'],
    mutationFn: async ({ dialog, projectId, data }) => {
      if (dialog.kind === 'request') {
        await sendMessageRequest(projectId, {
          text: data.requestText ?? '',
          attachments: data.attachments ?? [],
        });
        // The sent request is shown where it waits: the Messages list on "Your requests".
        openMessages({ tab: 'fromYou' });
      } else if (dialog.kind === 'edit') {
        if (data.attachments || data.removedAttachmentIds?.length)
          await editTaskMultipart(projectId, dialog.task.id, {
            title: data.title,
            description: data.description,
            finalize: data.finalize,
            requestId: data.requestId,
            attachments: data.attachments ?? [],
            removedAttachmentIds: data.removedAttachmentIds,
          });
        else
          await editTask(projectId, dialog.task.id, {
            title: data.title,
            description: data.description,
            body: data.body,
            acceptance: data.acceptance,
            resultPlans: data.resultPlans,
            finalize: data.finalize,
          });
      }
    },
    onSuccess: (_result, { projectId }) => refreshProject(projectId),
  });
  return useCallback(
    async (data: ActionDialogData) => {
      const { dialog, operation } = session;
      if (!dialog || !operation) return;
      await mutateAsync({ dialog, projectId: operation.projectId, data });
    },
    [mutateAsync, session],
  );
}
