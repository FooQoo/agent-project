import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { openWhenLoaded } from './openWhenLoaded';
import type { AppControllerModel, AppWorkspaceActions } from './contracts';
import { WriteError, moveTask } from './api';
import { useDialogSubmit, useReorder } from './useBoardWrites';
import { moveFailureNotice } from '../features/tasks/api/lifecycle';
import { useEpicWrites } from '../features/epics/editor/epicApi';
import { groupTasks } from '../features/tasks/model/board';
import type { ColumnName, ProjectId } from '../shared/api/contracts';
import { useLaneMove } from '../features/tasks/useLaneMove';
import { useBacklogMove } from '../features/backlog/useBacklogMove';
import { useBacklogSend } from '../features/backlog/useBacklogSend';
import { isProjectId } from './model/projectSession';

export function useAppActions(model: AppControllerModel) {
  const { t } = useTranslation();
  const {
    activeId,
    archiveLifecycle,
    archiveVisible,
    archivedSelected,
    dispatch,
    operations,
    refreshProject,
    session,
    setRequested,
    setRestoreFailure,
    tasks,
    viewMode,
  } = model;

  const openMessages = model.messages.panel.open;
  const [moveNotice, setMoveNotice] = useState<string | null>(null);
  const dismissMoveNotice = useCallback(() => setMoveNotice(null), []);
  const sendProjectId = isProjectId(activeId) ? activeId : null;
  const reloadBoard = () => {
    if (sendProjectId) refreshProject(sendProjectId);
  };
  const backlogSend = useBacklogSend(sendProjectId, tasks, reloadBoard);

  // One Backlog card moved by a plain drag or the details' button, apart from the batch send above.
  const backlogMove = useBacklogMove(sendProjectId, tasks, {
    apply: model.moveCard,
    settle: model.settleCard,
    revert: model.unmoveCard,
    onDone: reloadBoard,
    refusal: (cause) => moveFailureNotice(cause, t),
  });

  const moveLane = useLaneMove({
    write: (id, to, from) => moveTask(sendProjectId as ProjectId, id, to, from),
    apply: model.moveCard,
    settle: model.settleCard,
    revert: model.unmoveCard,
    onDone: reloadBoard,
    notify: setMoveNotice,
    refusal: (cause) => moveFailureNotice(cause, t),
  });

  const reorder = useReorder(refreshProject);
  const epicWrites = useEpicWrites(refreshProject);

  const workspace: AppWorkspaceActions = {
    onSelectProject: (projectId) => {
      void setRequested(projectId, { history: 'push' });
    },
    onBack: () => dispatch({ type: 'SET_VIEW_MODE', mode: 'board' }),
    onAdd: () => model.openDialog({ kind: 'request' }),
    onArchive: () => dispatch({ type: 'SET_VIEW_MODE', mode: 'archive' }),
    onDocs: () => {
      if (isProjectId(activeId)) model.setDocsProjectId(activeId);
    },
    onEpics: () => dispatch({ type: 'SET_VIEW_MODE', mode: 'epics' }),
    onSelectEpic: (id) => dispatch({ type: 'SELECT_EPIC', id }),
    onSelectArchiveCard: (id) => dispatch({ type: 'SELECT_ARCHIVE_CARD', id }),
    onSelectCard: (id) => dispatch({ type: 'SELECT_CARD', id }),
    onOpenEpic: (epicId) => {
      const epic = model.boardData?.epics.find((item) => item.id === epicId);
      if (!epic || !isProjectId(activeId)) return;
      openWhenLoaded(model.loadEpic(activeId, epic), (loaded) =>
        model.openDialog({ kind: 'epic', epic: loaded }),
      );
    },
    onMove: (task, column) => {
      if (!isProjectId(activeId) || session.operation !== null) return;
      void moveLane(task.id, task.taskStatus, column as ColumnName);
    },
    onMoveBacklog: (id, to) => {
      if (session.operation === null) backlogMove.request(id, to);
    },
    sendBusy: backlogSend.busy,
    onSendEpic: (epicId) =>
      session.operation === null &&
      backlogSend.request(
        tasks
          .filter((task) => task.taskStatus === 'backlog' && task.epic?.id === epicId)
          .map((task) => task.id),
        epicId,
      ),
    onSend: (ids, to) =>
      session.operation === null ? backlogSend.request(ids, undefined, to) : false,
    onReorder: (task, beforeId) => {
      if (!isProjectId(activeId)) return;
      reorder.mutate({ projectId: activeId, id: task.id, beforeId });
    },
    onArchiveDone: () => {
      if (!isProjectId(activeId) || session.operation !== null) return;
      operations.begin('archive-done', activeId);
      dispatch({ type: 'OPEN_BULK_ARCHIVE', projectId: activeId, targets: groupTasks(tasks).done });
    },
    onCloseEpic: () => dispatch({ type: 'SELECT_EPIC', id: null }),
    onEditEpic: () => {
      const epic = model.selectedEpic;
      if (!epic || !isProjectId(activeId)) return;
      openWhenLoaded(model.loadEpic(activeId, epic), (loaded) =>
        model.openDialog({ kind: 'epic', epic: loaded }),
      );
    },
    onDeleteEpic: async () => {
      const epic = model.selectedEpic;
      if (!epic || !activeId) return;
      await epicWrites(activeId, epic.id).remove();
    },
    onFinalizeEpic: async () => {
      const epic = model.selectedEpic;
      if (!epic || !activeId) return;
      await epicWrites(activeId, epic.id).leaveDraft();
    },
    onRetryEpics: () => {
      if (archiveVisible) model.archive.reload();
      else if (activeId) refreshProject(activeId);
    },
    onPreviewTask: () => {
      const card = archiveVisible ? archivedSelected : model.selected;
      if (!card || !isProjectId(activeId)) return;
      const archived = archiveVisible;
      openWhenLoaded(model.loadTask(activeId, card), (task) =>
        model.openDialog({ kind: 'edit', task, archived }),
      );
    },
    onTaskChanged: () => {
      if (activeId) refreshProject(activeId);
    },
    onRemove: async (action, task) => {
      if (!isProjectId(activeId)) return false;
      const projectId = activeId;
      const result = await archiveLifecycle.run(
        action,
        projectId,
        { taskId: task.id },
        { keepOperationOnError: true },
      );
      if (!result.ok) return false;
      refreshProject(projectId);
      if (session.viewMode === 'archive') model.archive.reload();
      return true;
    },
    onRestore: () => {
      if (!isProjectId(activeId) || archivedSelected === null) return;
      const projectId = activeId;
      const taskId = archivedSelected.id;
      setRestoreFailure(null);
      void archiveLifecycle.run('restore', projectId, { taskId }).then((result) => {
        if (!result.ok) {
          setRestoreFailure({ projectId, taskId });
          return;
        }
        dispatch({ type: 'SELECT_ARCHIVE_CARD', id: null });
        model.archive.reload();
        refreshProject(projectId);
      });
    },
    onCloseDetails: () => {
      dispatch({
        type: viewMode === 'archive' ? 'SELECT_ARCHIVE_CARD' : 'SELECT_CARD',
        id: null,
      });
      // The link that opened the task ends with it, so the same link can open it again; a task
      // opened from a summary result gives the Summary list back where it was left.
      model.clearResultLink?.();
      model.messages.panel.returnToList();
    },
  };

  const submitDialog = useDialogSubmit(session, refreshProject, openMessages);

  const confirmBulkArchive = useCallback(async () => {
    const bulkArchive = session.bulkArchive;
    if (!bulkArchive) return;
    const result = await archiveLifecycle.run(
      'archive-done',
      bulkArchive.projectId,
      { taskIds: bulkArchive.targets.map((task) => task.id) },
      { keepOperationOnError: true },
    );
    if (!result.ok) {
      throw new Error(
        result.error instanceof WriteError && result.error.status === 409
          ? t('archive.bulkConflict')
          : t('archive.error'),
      );
    }
    dispatch({ type: 'CLOSE_BULK_ARCHIVE' });
    refreshProject(bulkArchive.projectId);
  }, [archiveLifecycle, dispatch, refreshProject, session.bulkArchive, t]);

  const cancelBulkArchive = useCallback(() => {
    if (session.operation?.kind === 'archive-done') operations.end(session.operation.id);
    dispatch({ type: 'CLOSE_BULK_ARCHIVE' });
  }, [dispatch, operations, session.operation]);

  const closeOperation = useCallback(() => {
    if (session.operation) operations.end(session.operation.id);
  }, [operations, session.operation]);

  const savingChanged = useCallback(
    (saving: boolean) => {
      const id = session.operation?.id;
      if (id === undefined) return;
      if (saving) operations.saveStarted(id);
      else operations.saveFailed(id);
    },
    [operations, session.operation],
  );

  const openTaskFromEpic = useCallback(
    (taskId: number) => {
      const id = session.operation?.id;
      if (id !== undefined) operations.end(id);
      dispatch({ type: 'SET_VIEW_MODE', mode: 'board' });
      dispatch({ type: 'SELECT_CARD', id: taskId });
    },
    [dispatch, operations, session.operation],
  );

  const operationProjectId = session.operation?.projectId;
  const epicApiForOperation =
    session.dialog?.kind === 'epic' && operationProjectId
      ? epicWrites(operationProjectId, session.dialog.epic.id)
      : undefined;

  return {
    workspace,
    moveNotice: backlogSend.notice ?? backlogMove.notice ?? moveNotice,
    dismissMoveNotice: backlogSend.notice
      ? backlogSend.dismissNotice
      : backlogMove.notice
        ? backlogMove.dismissNotice
        : dismissMoveNotice,
    backlogSend,
    backlogMove,
    submitDialog,
    confirmBulkArchive,
    cancelBulkArchive,
    closeOperation,
    savingChanged,
    openTaskFromEpic,
    epicApiForOperation,
  };
}
