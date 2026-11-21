import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ColumnName, ProjectId } from '../../../shared/api/contracts';
import { moveFailureNotice } from '../../../features/tasks/api/lifecycle';
import { useWrite } from '../../../shared/data/react/use-write';
import { useLaneMove } from '../../../features/tasks/useLaneMove';
import type { MonitorTask, MonitorWrites } from '../contracts';
import type { useLaneOverlay } from './useLaneOverlay';

/**
 * What the lanes of the phone write: a move at once, an archive of one card, and the archive of the
 * whole Done lane. A move stays on top of the lanes while its save is in flight and until a newer read
 * shows the project, and an archive until a read shows the card gone. A refused move says so in a notice.
 */
export function useLaneWrites({
  writes,
  projectId,
  overlay,
  doneItems,
  closePending,
  onWritten,
}: {
  writes: MonitorWrites | undefined;
  projectId: string;
  overlay: ReturnType<typeof useLaneOverlay>;
  doneItems: readonly MonitorTask[];
  closePending: () => void;
  onWritten?: () => void;
}) {
  const { t } = useTranslation();
  const [notice, setNotice] = useState<string | null>(null);
  const dismissNotice = useCallback(() => setNotice(null), []);
  // A move by a person is made at once, with no dialog: the card shows in its new lane before the
  // server answers, and a refusal takes it back. A task that moved meanwhile is not moved.
  const move = useLaneMove({
    write: async (id, target, from) => {
      if (writes) await writes.move(projectId as ProjectId, id, target, from);
    },
    apply: overlay.move,
    settle: overlay.settle,
    revert: overlay.unmove,
    onDone: () => onWritten?.(),
    notify: setNotice,
    refusal: (cause) => moveFailureNotice(cause, t),
  });
  const moveNow = async (item: MonitorTask, target: ColumnName) => {
    if (!writes) return;
    await move(item.task.id, item.task.taskStatus, target);
  };
  const archive = useWrite<unknown, number>({
    mutationKey: ['phone-archive'],
    mutationFn: (id) => (writes ? writes.archive(projectId as ProjectId, id) : Promise.resolve()),
    onSuccess: (_result, id) => {
      overlay.hide([id]);
      closePending();
      onWritten?.();
    },
  });
  const archiveOne = async (item: MonitorTask) => {
    try {
      await archive.mutateAsync(item.task.id);
      return true;
    } catch {
      return false;
    }
  };
  // The Done lane's archive reads the project again whether or not it went through.
  const archiveDoneLane = useWrite<unknown, number[]>({
    mutationKey: ['phone-archive-done'],
    mutationFn: (ids) =>
      writes ? writes.archiveDone(projectId as ProjectId, ids) : Promise.resolve(),
    onSuccess: (_result, ids) => overlay.hide(ids),
    onSettled: () => onWritten?.(),
  });
  const archiveAll = async () => {
    await archiveDoneLane.mutateAsync(doneItems.map((item) => item.task.id));
    closePending();
  };
  return { moveNow, archiveOne, archiveAll, notice, dismissNotice };
}
