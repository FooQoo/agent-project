import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { TransientNotice } from './TransientNotice';

const meta = {
  title: 'Shared/UI/TransientNotice',
  component: TransientNotice,
  args: {
    message: 'Another change moved this task to Doing. It stays there.',
    onDismiss: () => {},
    durationMs: 3_600_000,
  },
} satisfies Meta<typeof TransientNotice>;

export default meta;
type Story = StoryObj<typeof meta>;

/** The notice is rendered at the body, so it is outside the Story's canvas. It reads as a status. */
export const Default: Story = {
  play: async () => {
    const notice = within(document.body).getByRole('status');
    await expect(notice).toHaveTextContent('Another change moved this task to Doing');
    await expect(within(notice).getByRole('button', { name: 'Dismiss message' })).toBeEnabled();
  },
};

/** A success has the check icon. */
export const Success: Story = {
  args: { message: 'The task was moved to Done.', tone: 'success' },
  play: async () => {
    await expect(within(document.body).getByRole('status')).toHaveAttribute('data-tone', 'success');
  },
};

/** A plain information has the info icon. */
export const Info: Story = {
  args: { message: 'The board was updated by another change.', tone: 'info' },
};

/** The Japanese text, in the light theme. */
export const Japanese: Story = {
  args: { message: 'タスクを移動できませんでした。元の列のままです。' },
  globals: { locale: 'ja', mode: 'light' },
};

/** The Japanese text and close label, in the dark theme. */
export const JapaneseDark: Story = {
  args: { message: 'タスクを移動できませんでした。元の列のままです。' },
  globals: { locale: 'ja', mode: 'dark' },
  play: async () => {
    await expect(
      within(document.body).getByRole('button', { name: 'メッセージを閉じる' }),
    ).toBeEnabled();
  },
};

/** On a phone the notice is at the top center. */
export const Phone: Story = {
  globals: { width: 'phone-390' },
};

/** The narrowest phone, in Japanese and in the dark theme. */
export const Phone320JapaneseDark: Story = {
  args: { message: 'タスクを移動できませんでした。元の列のままです。' },
  globals: { locale: 'ja', mode: 'dark', width: 'phone-320' },
};

/** Without a message nothing is drawn. */
export const Hidden: Story = {
  args: { message: null },
  play: async () => {
    await expect(within(document.body).queryByRole('status')).toBeNull();
  },
};
