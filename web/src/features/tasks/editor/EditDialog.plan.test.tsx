import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import '../../../shared/i18n';
import type { Task } from '../../../shared/api/contracts';
import type { PlanFieldArgs } from '../../../shared/ui/editor/planField';
import { EditDialog } from './EditDialog';
import type { ReadOnlyReason } from './PreviewStatus';

afterEach(cleanup);

const task = {
  id: 7,
  title: 'Shipped',
  description: 'Text',
  body: 'Text',
  acceptance: [],
  acceptanceError: false,
  plannedVersion: null,
  assignee: 'ai',
  taskStatus: 'done',
  agentStatus: 'idle',
  epic: null,
  draft: null,
  attachments: [],
  updatedAt: '',
} as unknown as Task;

function open(readOnlyReason?: ReadOnlyReason) {
  const Field = ({ disabled }: PlanFieldArgs) => (
    <button type="button" disabled={disabled}>
      plan picker
    </button>
  );
  render(
    <EditDialog
      task={task}
      onClose={() => {}}
      onSubmit={async () => {}}
      renderEpicChip={() => null}
      readOnlyReason={readOnlyReason}
      renderPlan={(args) => <Field {...args} />}
    />,
  );
  return screen.getByRole('button', { name: 'plan picker' });
}

describe('the planned version field of the task editor', () => {
  it('is enabled on a Done card', () => {
    expect((open() as HTMLButtonElement).disabled).toBe(false);
  });

  it('is enabled on an archived task, whose other fields are read-only', () => {
    expect((open('archived') as HTMLButtonElement).disabled).toBe(false);
  });

  it('stays disabled while the card runs', () => {
    expect((open('running') as HTMLButtonElement).disabled).toBe(true);
  });
});
