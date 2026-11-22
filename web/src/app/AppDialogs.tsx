import { Suspense } from 'react';
import { actionDialog, docsDialog } from './dialogs/lazyDialogs';
import type { AppDialogsProps } from './contracts';
import { useNarrowViewport } from '../shared/lib/viewport/useNarrowViewport';

export function AppDialogs({
  model,
  onSubmit,
  onCloseOperation,
  onSavingChange,
  onOpenTask,
  epicApi,
}: AppDialogsProps) {
  // The planned-version fields are on the PC board only.
  const narrow = useNarrowViewport();
  const { session, tasks } = model;
  const dialog = session.dialog;
  const operation = session.operation;

  return (
    <>
      {dialog && operation ? (
        <Suspense fallback={null}>
          <actionDialog.Component
            key={`${operation.id}:${operation.projectId}`}
            action={dialog}
            projectId={operation.projectId}
            onClose={onCloseOperation}
            onSavingChange={onSavingChange}
            tasks={tasks}
            epicApi={dialog.kind === 'epic' ? epicApi : undefined}
            onOpenTask={onOpenTask}
            onSubmit={onSubmit}
            subscribeChanges={model.subscribeChanges}
            loadTask={model.loadTask}
            loadEpic={model.loadEpic}
            isTaskListed={model.isTaskListed}
            onChanged={() => model.refreshProject(operation.projectId)}
            plan={!narrow}
          />
        </Suspense>
      ) : null}
      {model.docsProjectId !== null ? (
        <Suspense fallback={null}>
          <docsDialog.Component
            key={model.docsProjectId}
            projectId={model.docsProjectId}
            onClose={() => model.setDocsProjectId(null)}
            subscribeChanges={model.subscribeDocs}
          />
        </Suspense>
      ) : null}
    </>
  );
}
