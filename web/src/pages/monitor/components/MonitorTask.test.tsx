import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import '../../../shared/i18n';
import { withMonitorFixtureData } from '../fixture-provider';
import { createMonitorFixture, createMonitorFixtureProps } from '../mock';
import type { MonitorTask as MonitorTaskModel } from '../contracts';
import { MonitorTask } from './MonitorTask';

function waitingHumanTask(): MonitorTaskModel {
  const fixture = createMonitorFixture();
  const item = createMonitorFixtureProps(fixture).doing[0];
  if (!item) throw new Error('the fixture has no Doing task');
  return {
    ...item,
    task: {
      ...item.task,
      assignee: 'human',
      taskStatus: 'todo',
      agentStatus: 'idle',
      awaitingAnswer: true,
      openQuestionIds: ['Q-1'],
    },
  };
}

describe('MonitorTask', () => {
  it('shows the waiting mark and the answer button on a human task that waits for an answer', () => {
    const item = waitingHumanTask();
    const fixture = createMonitorFixture();
    render(
      withMonitorFixtureData(
        fixture,
        <MonitorTask item={item} section="todo" onOpenAnswer={() => undefined} />,
      ),
    );
    const card = document.querySelector('.monitor-card');
    expect(card?.getAttribute('data-awaiting-answer')).toBe('true');
    expect(card?.getAttribute('data-agent-status')).toBeNull();
    expect(screen.getByRole('button', { name: /answer/i })).toBeTruthy();
  });
});
