import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { selectLocale } from '../../src/shared/i18n';
import { MonitorTask } from '../../src/pages/monitor/components/MonitorTask';
import { stateLabel } from '../../src/features/tasks/model/format';
import type { MonitorTask as MonitorTaskModel } from '../../src/pages/monitor/contracts';
import { PROJECT_A, makeTask } from '../fixtures';

beforeEach(() => selectLocale('en'));

const lanes = ['backlog', 'todo', 'doing', 'done'] as const;
const owners = ['ai', 'human'] as const;

function renderCard(task: MonitorTaskModel['task']) {
  const section = task.taskStatus as (typeof lanes)[number];
  return render(
    <MonitorTask
      item={{ projectId: PROJECT_A.id, projectName: PROJECT_A.name, task }}
      section={section}
    />,
  );
}

describe('phone card state label', () => {
  for (const taskStatus of lanes) {
    for (const assignee of owners) {
      for (const awaitingAnswer of [false, true]) {
        it(`follows the PC rule for a ${assignee} card in ${taskStatus}${awaitingAnswer ? ' awaiting an answer' : ''}`, () => {
          const task = makeTask({
            id: 7,
            taskStatus,
            assignee,
            agentStatus: 'idle',
            awaitingAnswer,
          });
          const { container } = renderCard(task);
          const shown = container.querySelector('.status-label')?.textContent ?? null;
          expect(shown).toBe(stateLabel(task));
        });
      }
    }
  }

  it('shows who owns the card, as on the PC board', () => {
    renderCard(makeTask({ id: 8, taskStatus: 'doing', assignee: 'human' }));
    expect(screen.getByRole('img', { name: 'User' })).toBeTruthy();
  });
});
