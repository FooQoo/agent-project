import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import '../../shared/i18n';
import { AgentList } from './AgentList';
import type { Agent } from './api';

afterEach(cleanup);

const agents: Agent[] = [
  { id: 'A-1', name: 'Default agent', role: '', tool: null, isDefault: true, archivedAt: null },
  { id: 'A-2', name: 'Screens', role: 'UI', tool: 'codex', isDefault: false, archivedAt: null },
];

it('shows each tool and reports a change', async () => {
  const onChangeTool = vi.fn();
  render(
    <AgentList agents={agents} busy={false} onChangeTool={onChangeTool} onRegister={vi.fn()} />,
  );
  expect((screen.getByLabelText('Tool for Screens') as HTMLSelectElement).value).toBe('codex');
  expect((screen.getByLabelText('Tool for Default agent') as HTMLSelectElement).value).toBe('');
  await userEvent.selectOptions(screen.getByLabelText('Tool for Default agent'), 'claude-code');
  expect(onChangeTool).toHaveBeenCalledWith('A-1', 'claude-code');
});

it('registers an agent without a tool and clears the form', async () => {
  const onRegister = vi.fn();
  render(<AgentList agents={agents} busy={false} onChangeTool={vi.fn()} onRegister={onRegister} />);
  const register = screen.getByRole('button', { name: 'Register agent' });
  expect((register as HTMLButtonElement).disabled).toBe(true);
  await userEvent.type(screen.getByLabelText('Agent name'), '  CLI ');
  await userEvent.click(register);
  expect(onRegister).toHaveBeenCalledWith({ name: 'CLI', role: '', tool: null });
  expect((screen.getByLabelText('Agent name') as HTMLInputElement).value).toBe('');
});
