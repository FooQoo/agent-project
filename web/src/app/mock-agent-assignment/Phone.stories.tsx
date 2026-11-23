import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { modalDocsParameters } from '../../../.storybook/support/modal-docs';
import { AssignmentScreen } from './AssignmentScreen';

// The phone side: lane tabs with agent avatars in the cards, and the same operations as bottom sheets. Connection codes are
// shown only on the PC, so the phone's agents sheet points there instead of offering the switch.
const meta = {
  title: 'Mocks/AgentAssignment/Phone',
  component: AssignmentScreen,
  args: { variant: 'phone' },
  parameters: { storyFrame: 'monitor', layout: 'fullscreen', ...modalDocsParameters },
  globals: { width: 'phone-375' },
} satisfies Meta<typeof AssignmentScreen>;
export default meta;
type Story = StoryObj<typeof meta>;

/** The Doing tab: each card carries only its agent's small avatar; no agent list on the board. */
export const Board: Story = {
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getAllByRole('tab')).toHaveLength(4);
    await expect(within(canvasElement).queryByRole('region', { name: /Agents|担当/ })).toBeNull();
  },
};

/** A Doing card in the detail drawer: the agent's avatar sits in the status row. */
export const CardDetail: Story = {
  args: { initial: { openId: 28 } },
};

/** Handover as a sheet: the choice list, what carries over and what stops. */
export const ReassignChoose: Story = {
  args: { initial: { openId: 28, dialog: 'reassign', to: 'A-1' } },
};

/** The new agent has no parent session: one more confirmation before the handover. */
export const ReassignUnconnected: Story = {
  args: { initial: { openId: 28, dialog: 'reassign', to: 'A-3' } },
};

/** After the handover on the Doing tab, with the toast. */
export const AfterReassign: Story = {
  args: { initial: { done: 'reassign', to: 'A-1' } },
};

/** The epic tab of the detail drawer: the epic's unassigned cards and the way to assign them. */
export const EpicTab: Story = {
  args: { initial: { openId: 28, tab: 'epic' } },
};

/** Epic bulk assign as a sheet, reached from the epic tab of the detail. */
export const EpicBulkConfirm: Story = {
  args: { initial: { openId: 28, tab: 'epic', dialog: 'bulk', epicId: 'LOGIN-1', to: 'A-1' } },
};

/** After the bulk assign on the Backlog tab. */
export const AfterBulk: Story = {
  args: { initial: { done: 'bulk', epicId: 'LOGIN-1', to: 'A-1', lane: 'backlog' } },
};

/** Agents sheet: names, roles and states; the connection code is shown only on the PC. */
export const AgentsSettings: Story = {
  args: { initial: { dialog: 'agents' } },
};
