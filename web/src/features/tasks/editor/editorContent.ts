import type { AcceptanceItem, Task } from '../../../shared/api/contracts';
import type { StoredAttachment } from '../../../shared/ui/editor/attachmentTypes';
import { normalizeNewlines } from '../../../shared/ui/editor/limits';
import { plansOf, type ResultPlanItem } from '../result-plans/model/resultPlans';

/** The savable fields of a card: what the editor shows and what the server holds. */
export interface EditorContent {
  title: string;
  description: string;
  acceptance: AcceptanceItem[];
  resultPlans: ResultPlanItem[];
  attachments: StoredAttachment[];
}

/**
 * What the editor reads from a card. A card whose frontmatter cannot be read is edited as its whole
 * text, so its description is the whole file and its conditions are not part of the content.
 */
export function contentOf(task: Task, whole: boolean): EditorContent {
  return {
    title: task.title,
    description: normalizeNewlines(whole ? task.description : task.body),
    acceptance: whole ? [] : task.acceptance,
    resultPlans: whole ? [] : plansOf(task),
    attachments: (task.attachments ?? []).map(({ id, name, mediaType, size, createdAt }) => ({
      id,
      name,
      mediaType,
      size,
      createdAt,
    })),
  };
}

/**
 * The savable fields as one string, compared by content. Dates and states are left out on purpose:
 * an agent changing only those must not look like a conflicting edit.
 */
export function snapshotOf(content: {
  title: string;
  description: string;
  acceptance: readonly AcceptanceItem[];
  resultPlans?: readonly ResultPlanItem[];
  attachmentIds: readonly string[];
}): string {
  return JSON.stringify([
    content.title,
    content.description,
    content.acceptance.map(({ id, text, done }) => [id, text, done]),
    (content.resultPlans ?? []).map(({ id, content: what, condition, final }) => [
      id,
      what,
      condition,
      final,
    ]),
    content.attachmentIds,
  ]);
}

export function snapshotOfContent(content: EditorContent): string {
  return snapshotOf({ ...content, attachmentIds: content.attachments.map((a) => a.id) });
}
