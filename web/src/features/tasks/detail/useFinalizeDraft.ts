import type { ProjectId } from '../../../shared/api/contracts';
import { useWrite } from '../../../shared/data/react/use-write';
import { setDraft } from '../api/edit';

const KEY = ['finalize-draft'] as const;

/** Ends a person's draft of a card (the edit lock), one request at a time; a failure is kept to show. */
export function useFinalizeDraft(projectId: ProjectId, taskId: number, onChanged?: () => void) {
  const mutation = useWrite({
    mutationKey: KEY,
    mutationFn: () => setDraft(projectId, taskId, null),
    onSuccess: () => onChanged?.(),
  });
  const finalizeDraft = async () => {
    if (mutation.isWriting()) return;
    await mutation.mutateAsync().catch(() => {});
  };
  return { draftSaving: mutation.isPending, draftFailed: mutation.isError, finalizeDraft };
}
