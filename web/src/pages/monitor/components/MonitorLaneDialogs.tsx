import { useTranslation } from 'react-i18next';
import { BulkArchiveDialog } from '../../../features/tasks/archive/BulkArchiveDialog';
import type { MonitorTask } from '../contracts';
import { ArchiveListSheet, type ArchiveSheetData } from './ArchiveListSheet';
import './monitor-lanes.css';

/** The confirmation the lane view is waiting on: nothing changes until the person agrees. A move asks nothing. */
export type LanePending =
  | { kind: 'archive'; item: MonitorTask }
  /** The list of archived tasks, opened from the bar's Archive item. */
  | { kind: 'list'; revealed?: number }
  | null;

/** The bar item a sheet of the lanes belongs to. */
export function pendingNavItem(pending: LanePending): 'archive' | null {
  return pending?.kind === 'list' ? 'archive' : null;
}

export interface MonitorLaneDialogsProps {
  pending: LanePending;
  projectName: string;
  onCancel: () => void;
  /** Resolves with false (or throws) when it failed, and the confirmation says so. */
  onArchive: (item: MonitorTask) => Promise<boolean>;
  /** What the archive list shows; without it the list is not offered. */
  archive?: ArchiveSheetData;
  /** Restores a task from the archive list to the lane it was archived from. */
  onRestore?: (taskId: number) => Promise<void>;
}

/**
 * The sheets of the lane view above the board: the archive question of a card, and the list of
 * archived tasks. The Done lane's archive is a selection of its own, not a sheet.
 */
export function MonitorLaneDialogs({
  pending,
  projectName,
  onCancel,
  onArchive,
  archive,
  onRestore,
}: MonitorLaneDialogsProps) {
  const { t } = useTranslation();
  if (pending === null) return null;
  if (pending.kind === 'list')
    return archive && onRestore ? (
      <ArchiveListSheet
        {...archive}
        projectName={projectName}
        initialRevealed={pending.revealed ?? null}
        onRestore={onRestore}
        onClose={onCancel}
      />
    ) : null;
  const item = pending.item;
  return (
    <BulkArchiveDialog
      projectName={projectName}
      targets={[item.task]}
      saving={false}
      title={t('monitor.lanes.archiveTitle')}
      description={t('monitor.lanes.archiveDescription', {
        id: item.task.id,
        title: item.task.title,
      })}
      submitLabel={t('monitor.lanes.archiveSubmit')}
      onConfirm={async () => {
        if (!(await onArchive(item))) throw new Error(t('archive.error'));
      }}
      onCancel={onCancel}
    />
  );
}
