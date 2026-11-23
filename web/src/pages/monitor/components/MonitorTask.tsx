import { useTranslation } from 'react-i18next';
import { formatShort, stateLabel } from '../../../features/tasks/model/format';
import { TaskCardContent, waitsOnQuestion } from '../../../features/tasks/card/TaskCard';
import { StatusLabel } from '../../../features/tasks/status/StatusLabel';
import { AcceptanceProgress } from '../../../features/tasks/acceptance/AcceptanceProgress';
import { DecisionAnswerButton } from '../../../features/tasks/answer/DecisionAnswerButton';
import { AssigneeLabel } from '../../../features/tasks/assignee/AssigneeLabel';
import { EpicLabel } from '../../../features/epics/label/EpicLabel';
import { PlanTag } from '../../../features/versions/ui/PlanContext';
import type { MonitorTask as MonitorTaskModel, MonitorViewProps } from '../contracts';
import type { MonitorSection } from './MonitorSectionHeader';
import { completedAt } from '../model/sort';

export function MonitorTask({
  item,
  section,
  onOpenDetail,
  onOpenAnswer,
}: {
  item: MonitorTaskModel;
  section: MonitorSection;
  onOpenDetail?: MonitorViewProps['onOpenDetail'];
  /** The card's answer button was pressed; the sheet is drawn outside the cards. */
  onOpenAnswer?: (item: MonitorTaskModel) => void;
}) {
  const { t } = useTranslation();
  const task = item.task;
  const when =
    section === 'done'
      ? completedAt(item)
      : section === 'todo'
        ? task.createdAt
        : task.agentStatusChangedAt;
  // The PC board and the phone share one rule for the state label (stateLabel).
  const state = stateLabel(task, t);
  const showsState = state !== null && !task.awaitingAnswer;
  const pending = task.awaitingAnswer;
  const label = [
    `#${task.id}`,
    t(`monitor.status.${section}`),
    state ?? '',
    task.title,
    pending ? task.pendingReason : '',
  ]
    .filter(Boolean)
    .join(' ');
  return (
    <article
      className={`monitor-card monitor-card--${section}`}
      aria-label={label}
      data-task-id={task.id}
      data-agent-status={showsState ? task.agentStatus : undefined}
      data-awaiting-answer={pending ? 'true' : undefined}
    >
      <span className="monitor-card__top">
        {/* The guide's sample cards have no number of their own; the span keeps the row's layout. */}
        <span className="monitor-card__id">{task.id > 0 ? `#${task.id}` : ''}</span>
        <AssigneeLabel assignee={task.assignee} />
        <span className="monitor-card__indicators">
          {task.taskStatus === 'backlog' && (task.openQuestionIds?.length ?? 0) > 0 && (
            <span className="card__open-question">{t('backlog.openQuestion')}</span>
          )}
          {state !== null && (
            <StatusLabel
              kind="agent"
              status={task.agentStatus}
              awaitingAnswer={task.awaitingAnswer}
            />
          )}
          <AcceptanceProgress task={task} />
        </span>
      </span>
      <TaskCardContent
        task={task}
        showPendingReason={pending && !waitsOnQuestion(task)}
        titleClassName="monitor-card__title"
        reasonClassName="monitor-card__reason"
        titleActionDisabled={!onOpenDetail}
        onTitleClick={() => onOpenDetail?.(item, 'description')}
      >
        {task.epic && (
          <EpicLabel
            epic={task.epic}
            className="monitor-card__epic"
            disabled={!onOpenDetail}
            onClick={() => onOpenDetail?.(item, 'epic')}
          />
        )}
      </TaskCardContent>
      <span className="monitor-card__meta">
        <span className="monitor-card__plan">
          <PlanTag planned={task.plannedVersion ?? null} />
        </span>
        <span className="monitor-card__time">
          {when
            ? formatShort(when)
            : section === 'done'
              ? t('monitor.completedUnknown')
              : formatShort(task.agentStatusChangedAt)}
        </span>
      </span>
      {pending && waitsOnQuestion(task) && onOpenAnswer && (
        <div className="monitor-card__request">
          <DecisionAnswerButton taskId={task.id} onClick={() => onOpenAnswer(item)} />
        </div>
      )}
    </article>
  );
}
