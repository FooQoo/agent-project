import { useCallback } from 'react';
import { addEpicAttachments, deleteEpic, deleteEpicAttachment, updateEpic } from '../api';
import type { Epic, ProjectId } from '../../../shared/api/contracts';
import { useWrite } from '../../../shared/data/react/use-write';
import type { EditorAttachment } from '../../../shared/ui/editor/attachmentTypes';

/**
 * What the last request of a save carries: the name, the color, and the whole text (the Markdown
 * body, or the whole document when the stored order cannot be read). `draft: null` finalizes
 * the draft the dialog entered; it is left out when the epic was a draft before the dialog opened.
 */
export type EpicSaveInput = { name: string; color: string; draft?: null } & (
  | { document: string }
  | { body: string }
);

/** The writes an epic dialog makes, one at a time and in the order used by the dialog. */
export interface EpicApi {
  /** Adds the new images to the epic; `requestId` makes a retry the same write. */
  addImages(files: EditorAttachment[], requestId: string): Promise<void>;
  removeImage(attachmentId: string): Promise<void>;
  /** Saves the card order of the epic: the IDs of its tasks, first is first. */
  saveWaves(waves: number[][]): Promise<void>;
  /** The last request of a save: name, color, text, and finalizing the draft. */
  save(input: EpicSaveInput): Promise<void>;
  /** Makes the epic a draft and returns what the server holds then. */
  enterDraft(): Promise<Epic>;
  /** Takes the draft back (discard). An epic has no history, so nothing records the discard. */
  leaveDraft(): Promise<void>;
  /** Deletes the epic. The server refuses while a task still names it. */
  remove(): Promise<void>;
}

/**
 * The writes of epic dialogs. Each write is one mutation, so two at once are each tracked; after
 * one that went through, `changed` runs with the project it was made in, so the board shows what
 * was saved. Returns the writes of one epic, bound to the project the dialog was opened in.
 */
export function useEpicWrites(changed: (projectId: ProjectId) => void) {
  const { mutateAsync } = useWrite<
    unknown,
    { projectId: ProjectId; write: () => Promise<unknown> }
  >({
    mutationKey: ['epic-write'],
    mutationFn: ({ write }) => write(),
    onSuccess: (_result, { projectId }) => changed(projectId),
  });
  return useCallback(
    (projectId: ProjectId, epicId: string): EpicApi => {
      const done = async <T>(write: () => Promise<T>) =>
        (await mutateAsync({ projectId, write })) as T;
      return {
        addImages: async (files, requestId) => {
          await done(() => addEpicAttachments(projectId, epicId, files, requestId));
        },
        removeImage: async (attachmentId) => {
          await done(() => deleteEpicAttachment(projectId, epicId, attachmentId));
        },
        saveWaves: async (waves) => {
          await done(() => updateEpic(projectId, epicId, { waves }));
        },
        save: async (input) => {
          await done(() => updateEpic(projectId, epicId, input));
        },
        enterDraft: async () =>
          (await done(() => updateEpic(projectId, epicId, { draft: 'human' }))).epic,
        leaveDraft: async () => {
          await done(() => updateEpic(projectId, epicId, { draft: null }));
        },
        remove: async () => {
          await done(() => deleteEpic(projectId, epicId));
        },
      };
    },
    [mutateAsync],
  );
}
