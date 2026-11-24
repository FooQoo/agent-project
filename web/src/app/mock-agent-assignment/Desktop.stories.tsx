import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { modalDocsParameters } from '../../../.storybook/support/modal-docs';
import { AssignmentScreen } from './AssignmentScreen';

// A visual mock of agent assignment (SPEC 20_AGENT_ASSIGNMENT). Fixed data held in memory, no product behavior.
const meta = {
  title: 'Mocks/AgentAssignment/Desktop',
  component: AssignmentScreen,
  parameters: { layout: 'fullscreen', ...modalDocsParameters },
} satisfies Meta<typeof AssignmentScreen>;
export default meta;
type Story = StoryObj<typeof meta>;

/** The board: every AI card carries the agent's avatar (the default agent unless changed); the human card shows none. */
export const Board: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    // The board holds no agent list: agents are managed in Settings.
    await expect(canvas.queryByRole('region', { name: /Agents|担当/ })).toBeNull();
    await expect(canvas.queryByRole('button', { name: /Manage agents|担当の設定/ })).toBeNull();
    // Every AI card carries the default agent's avatar, named by its label; no card says "Unassigned".
    await expect(
      canvas.getAllByRole('img', { name: /^(エージェント|Agent)$/ }).length,
    ).toBeGreaterThan(0);
    await expect(canvas.queryByText(/未割り当て|Unassigned/)).toBeNull();
  },
};

/** A Doing card open in the existing detail panel: the agent's avatar sits in the status row. */
export const CardDetail: Story = {
  args: { initial: { openId: 28 } },
};

/** The card detail with the assignee menu open under the avatar: every agent with its connection state. */
export const AssigneeMenuOpen: Story = {
  args: { initial: { openId: 28, menu: true } },
};

/** After choosing another agent in the menu: the card carries the new agent and the toast gives the result. */
export const AfterReassign: Story = {
  args: { initial: { openId: 28, done: 'reassign', to: 'A-1' } },
};

/** Settings > Agents (opened from the settings button): name, role, state and parent session of each agent; the three connection states. */
export const AgentsSettings: Story = {
  args: { initial: { dialog: 'agents' } },
};

/** Registering an agent: name and role, with the rules beside the fields. */
export const AgentsAdd: Story = {
  args: { initial: { dialog: 'agentsAdd' } },
};

/** Switching a connection: the target and what happens to the current session, before any code is shown. */
export const SwitchConfirm: Story = {
  args: { initial: { dialog: 'switch', switchAgent: 'A-1' } },
};

/** After "Show the switch code": the one-time command for the new parent session. */
export const SwitchCode: Story = {
  args: { initial: { dialog: 'switchCode', switchAgent: 'A-1' } },
};
