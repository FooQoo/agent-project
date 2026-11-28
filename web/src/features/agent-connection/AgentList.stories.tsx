import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, within } from 'storybook/test';
import { AgentList } from './AgentList';
import type { Agent } from './api';

// The agents of the "Agents" settings dialog: each agent's tool, and the form that registers one.
const agent = (id: string, name: string, role: string, tool: Agent['tool']): Agent => ({
  id,
  name,
  role,
  tool,
  isDefault: id === 'A-1',
  archivedAt: null,
});

const agents = [
  agent('A-1', 'Default agent', '', null),
  agent('A-2', 'Screens', 'Builds the screens', 'claude-code'),
  agent('A-3', 'Server', 'Rust API and storage for the board and its remote entrance', 'codex'),
];

const meta = {
  title: 'Settings/AgentList',
  component: AgentList,
  args: { agents, busy: false, onChangeTool: fn(), onRegister: fn() },
  decorators: [
    (Story) => (
      <div className="modal settings-dialog" style={{ maxWidth: 560, padding: 24 }}>
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof AgentList>;
export default meta;
type Story = StoryObj<typeof meta>;

/** Desktop: an agent with no tool, one with Claude Code, one with Codex. */
export const Desktop: Story = {
  play: async ({ canvasElement, args }) => {
    const view = within(canvasElement);
    await expect(view.getByLabelText('Tool for Screens')).toHaveValue('claude-code');
    await userEvent.selectOptions(view.getByLabelText('Tool for Default agent'), 'codex');
    await expect(args.onChangeTool).toHaveBeenCalledWith('A-1', 'codex');
  },
};

/** A phone-width view of the same list. */
export const Phone: Story = {
  parameters: { viewport: { defaultViewport: 'mobile' } },
  decorators: [
    (Story) => (
      <div style={{ maxWidth: 360, padding: 16 }}>
        <Story />
      </div>
    ),
  ],
};

/** Registering an agent with a tool sends the trimmed name, the role, and the tool. */
export const Register: Story = {
  args: { agents: [agents[0] as Agent] },
  play: async ({ canvasElement, args }) => {
    const view = within(canvasElement);
    await userEvent.type(view.getByLabelText('Agent name'), ' CLI ');
    await userEvent.type(view.getByLabelText('Role'), 'Command line');
    await userEvent.selectOptions(view.getByLabelText('Tool'), 'codex');
    await userEvent.click(view.getByRole('button', { name: 'Register agent' }));
    await expect(args.onRegister).toHaveBeenCalledWith({
      name: 'CLI',
      role: 'Command line',
      tool: 'codex',
    });
  },
};

/** A write is on its way: the button and the tool choices wait. */
export const Saving: Story = {
  args: { busy: true },
};
