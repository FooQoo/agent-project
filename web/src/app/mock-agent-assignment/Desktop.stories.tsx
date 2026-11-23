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

/** The board with the agent strip: each agent's name, state and open cards; the epics with unassigned cards. */
export const Board: Story = {
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole('region', { name: /Agents|担当/ })).toBeVisible();
  },
};

/** A Doing card open in the existing detail panel, with the assignment section above it. */
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

/** Bulk assign on an epic: only the unassigned cards change; the others stay with the reason. */
export const EpicBulkConfirm: Story = {
  args: { initial: { dialog: 'bulk', epicId: 'LOGIN-1', to: 'A-1' } },
};

/** After the bulk assign: the assigned cards are marked and the toast names them. */
export const AfterBulk: Story = {
  args: { initial: { done: 'bulk', epicId: 'LOGIN-1', to: 'A-1', lane: 'backlog' } },
};

/** Settings > Agents: name, role, state and parent session of each agent; the three connection states. */
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
