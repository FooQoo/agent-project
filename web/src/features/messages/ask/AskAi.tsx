import type { MutableRefObject } from 'react';
import type { ProjectId } from '../../../shared/api/contracts';
import { AskAiDialog, type AskAiInput } from './AskAiDialog';
import type { PickerTask } from './pickerTasks';
import { useWrite } from '../../../shared/data/react/use-write';
import { sendMessageRequest } from './sendRequest';

export interface AskAiProps {
  projectId: ProjectId;
  /** The dialog is done: sent, or closed. */
  onClose: () => void;
  /** The request was saved. The Summary refreshes through the live update. */
  onSent?: () => void;
  /** A draft the text starts with. */
  initialText?: string;
  /** Receives the dialog's guarded close (see `AskAiDialog`). */
  closeGuard?: MutableRefObject<((after: () => void) => void) | null>;
  /** The project's tasks, for "Attach task". */
  tasks?: readonly PickerTask[];
}

/** "Ask AI" that sends its own request. A modal on the board (`kind="dialog"`) or a sheet. */
export function AskAi({
  kind = 'dialog',
  projectId,
  onClose,
  onSent,
  initialText,
  closeGuard,
  tasks,
}: AskAiProps & { kind?: 'dialog' | 'sheet' }) {
  const send = useWrite<void, AskAiInput>({
    mutationKey: ['ask-ai', projectId],
    mutationFn: (input) => sendMessageRequest(projectId, input),
  });
  return (
    <AskAiDialog
      kind={kind}
      initialText={initialText}
      closeGuard={closeGuard}
      tasks={tasks}
      onClose={onClose}
      onSubmit={async (input) => {
        await send.mutateAsync(input);
        onSent?.();
      }}
    />
  );
}

/**
 * "Ask AI" as a bottom sheet, for the phone. The messages band opens it from its "Ask AI" button:
 * `{asking && <AskAiSheet projectId={id} onClose={() => setAsking(false)} />}`.
 */
export function AskAiSheet(props: AskAiProps) {
  return <AskAi kind="sheet" {...props} />;
}
