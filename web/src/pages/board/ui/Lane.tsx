import { useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { columnLabel } from '../../../features/tasks/model/format';
import { useScrollCue } from '../../../shared/lib/scroll/useScrollCue';
import type { TaskSummary as TaskModel, ColumnName, EpicRef } from '../../../shared/api/contracts';
import { EpicLabel } from '../../../features/epics/label/EpicLabel';
import { TaskCard } from '../../../features/tasks/card/TaskCard';
import { LaneHeader } from './LaneHeader';
import { useDroppable } from '@dnd-kit/react';
import type { ProjectId } from '../../../shared/api/contracts';
import { BacklogBar } from '../../../features/backlog/BacklogBar';
import type { LaneSelection } from '../../../features/backlog/useSelection';
import type { SelectionAction } from '../../../features/backlog/selectionAction';

export interface LaneProps {
  column: ColumnName;
  /** A card is being dragged over the lane or one of its cards. */
  hovered?: boolean;
  tasks: TaskModel[];
  selectedId: number | null;
  onSelect: (id: number) => void;
  onOpenEpic?: (epicId: string) => void;
  projectId?: ProjectId;
  onAnswer?: (taskId: number) => void;
  /**
   * The selection mode of the lane and what it does with the picked cards: the Backlog sends them, the
   * Done lane archives them.
   */
  picker?: {
    action: SelectionAction;
    selection: LaneSelection;
    busy: boolean;
    onSubmit: (ids: number[]) => void;
    /** The picked cards are being dragged: the card under the pointer. */
    draggingId: number | null;
  };
}

export function Lane({
  column,
  hovered = false,
  tasks,
  selectedId,
  onSelect,
  onOpenEpic,
  projectId,
  onAnswer,
  picker,
}: LaneProps) {
  const { t } = useTranslation();
  const listRef = useRef<HTMLDivElement>(null);
  const more = useScrollCue(listRef, tasks);
  const { ref, isDropTarget } = useDroppable({
    id: `lane-${column}`,
    data: { column, group: column },
  });
  // The card draws no epic label of its own; the board puts the epic's label on it.
  const renderEpic = (epic: EpicRef) => (
    <EpicLabel
      epic={epic}
      className="card__epic"
      stopPropagation
      onClick={() => onOpenEpic?.(epic.id)}
    />
  );
  const picking = picker?.selection.selecting === true;
  return (
    <section
      ref={ref}
      className="lane"
      data-column={column}
      data-drop-target={isDropTarget || hovered ? 'true' : 'false'}
      aria-label={columnLabel(column, t)}
    >
      <LaneHeader
        column={column}
        count={tasks.length}
        selecting={picker?.selection.selecting}
        onToggleSelect={
          picker
            ? picker.selection.selecting
              ? picker.selection.leave
              : picker.selection.enter
            : undefined
        }
      />
      <div className="lane__scroll" data-more={more ? 'true' : 'false'}>
        <div className="lane__cards" ref={listRef}>
          {tasks.length === 0 && column === 'backlog' && (
            <p className="lane__empty">{t('backlog.empty')}</p>
          )}
          {tasks.map((task, index) => (
            <TaskCard
              key={task.id}
              task={task}
              index={index}
              selected={task.id === selectedId}
              onSelect={picking ? picker.selection.toggle : onSelect}
              selection={
                picking
                  ? {
                      checked: picker.selection.picked.includes(task.id),
                      onToggle: () => picker.selection.toggle(task.id),
                      drag:
                        picker.draggingId !== null &&
                        picker.selection.picked.includes(picker.draggingId) &&
                        picker.selection.picked.includes(task.id)
                          ? {
                              count: picker.selection.picked.length,
                              lead: task.id === picker.draggingId,
                            }
                          : undefined,
                    }
                  : undefined
              }
              renderEpic={renderEpic}
              projectId={projectId}
              onAnswer={onAnswer}
            />
          ))}
        </div>
      </div>
      {picking && (
        <BacklogBar
          action={picker.action}
          count={picker.selection.picked.length}
          busy={picker.busy}
          onCancel={picker.selection.leave}
          onSend={() => picker.onSubmit(picker.selection.picked)}
        />
      )}
    </section>
  );
}
