import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { modalDocsParameters } from '../../../.storybook/support/modal-docs';
import { AssignmentScreen } from './AssignmentScreen';

// The phone side: lane tabs, the agent strip, and the same operations as bottom sheets. Connection codes are
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

/** The Doing tab with the agent strip and each card's agent. */
export const Board: Story = {
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getAllByRole('tab')).toHaveLength(4);
  },
};

/** A Doing card in the detail drawer, with the assignment section above the existing detail. */
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

/** Epic bulk assign as a sheet: the cards that change and the ones that stay. */
export const EpicBulkConfirm: Story = {
  args: { initial: { dialog: 'bulk', epicId: 'LOGIN-1', to: 'A-1' } },
};

/** After the bulk assign on the Backlog tab. */
export const AfterBulk: Story = {
  args: { initial: { done: 'bulk', epicId: 'LOGIN-1', to: 'A-1', lane: 'backlog' } },
};

/** Agents sheet: names, roles and states; the connection code is shown only on the PC. */
export const AgentsSettings: Story = {
  args: { initial: { dialog: 'agents' } },
};
