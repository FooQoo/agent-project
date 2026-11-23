import type { ProjectId, ResultPlanView } from '../../../shared/api/contracts';
import { projectPath } from '../../../shared/api/http';
import { postForm } from '../../../shared/api/write';
import type { EditorAttachment } from '../../../shared/ui/editor/attachmentTypes';

/** What the person gives when registering the real result of a plan. */
export interface PlanDelivery {
  summary: string;
  link: string;
  attachments: readonly EditorAttachment[];
}

/**
 * Registers the result of a plan: `POST .../tasks/{id}/results/multipart` with the plan, the
 * summary and link when given (the summary otherwise reads as the plan's content), and one
 * `files` part per picture. A failed request changes nothing, so it can be sent again as it is.
 */
export function deliverPlan(
  projectId: ProjectId,
  taskId: number,
  plan: Pick<ResultPlanView, 'id'>,
  delivery: PlanDelivery,
) {
  const pictures = delivery.attachments.filter((a) => a.source === 'local' && a.file);
  const form = new FormData();
  form.append(
    'metadata',
    JSON.stringify({
      plan: plan.id,
      ...(delivery.summary.trim() === '' ? {} : { summary: delivery.summary.trim() }),
      ...(delivery.link.trim() === '' ? {} : { link: delivery.link.trim() }),
      attachments: pictures.map((a) => ({ id: a.id, name: a.name })),
    }),
  );
  for (const picture of pictures) form.append('files', picture.file as File, picture.name);
  return postForm<{ created: boolean }>(
    `${projectPath(projectId)}/tasks/${taskId}/results/multipart`,
    form,
  );
}
