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

/** The board with no agent list: each card carries only its agent's small avatar in the meta row. */
export const Board: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    // The board holds no agent list and no bulk bar: agents are managed in Settings.
    await expect(canvas.queryByRole('region', { name: /Agents|担当/ })).toBeNull();
    await expect(canvas.queryByRole('button', { name: /Manage agents|担当の設定/ })).toBeNull();
  },
};

/** A Doing card open in the existing detail panel: the agent's avatar sits in the status row. */
export const CardDetail: Story = {
  args: { initial: { openId: 28 } },
};

/** Handover in the middle of the work: nothing chosen yet, what carries over and what stops are listed. */
export const ReassignChoose: Story = {
  args: { initial: { openId: 28, dialog: 'reassign' } },
};

/** The new agent has no parent session: the handover needs one more confirmation. */
export const ReassignUnconnected: Story = {
  args: { initial: { openId: 28, dialog: 'reassign', to: 'A-3' } },
};

/** After the handover: the card carries the new agent's name, the toast gives the result. */
export const AfterReassign: Story = {
  args: { initial: { openId: 28, done: 'reassign', to: 'A-1' } },
};

/** The epic tab of the detail: the epic's unassigned cards and the way to assign them (the only entry to bulk assign). */
export const EpicTab: Story = {
  args: { initial: { openId: 28, tab: 'epic' } },
};

/** Bulk assign from the epic tab of the detail: only the unassigned cards change; the others stay with the reason. */
export const EpicBulkConfirm: Story = {
  args: { initial: { openId: 28, tab: 'epic', dialog: 'bulk', epicId: 'LOGIN-1', to: 'A-1' } },
};

/** After the bulk assign: the assigned cards carry the agent's avatar and the toast names them. */
export const AfterBulk: Story = {
  args: { initial: { done: 'bulk', epicId: 'LOGIN-1', to: 'A-1', lane: 'backlog' } },
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
