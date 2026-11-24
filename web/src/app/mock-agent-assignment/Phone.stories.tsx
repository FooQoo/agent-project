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

/** The Doing tab: every AI card carries the agent's avatar; no agent list on the board. */
export const Board: Story = {
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getAllByRole('tab')).toHaveLength(4);
    await expect(within(canvasElement).queryByRole('region', { name: /Agents|担当/ })).toBeNull();
    await expect(within(canvasElement).queryByText(/未割り当て|Unassigned/)).toBeNull();
  },
};

/** A Doing card in the detail drawer: the agent's avatar sits in the status row. */
export const CardDetail: Story = {
  args: { initial: { openId: 28 } },
};

/** The assignee sheet opened from the detail: the same list as the PC menu. */
export const AssigneeMenuOpen: Story = {
  args: { initial: { openId: 28, menu: true } },
};

/** After the handover on the Doing tab, with the toast. */
export const AfterReassign: Story = {
  args: { initial: { done: 'reassign', to: 'A-1' } },
};

/** Agents sheet: names, roles and states; the connection code is shown only on the PC. */
export const AgentsSettings: Story = {
  args: { initial: { dialog: 'agents' } },
};
