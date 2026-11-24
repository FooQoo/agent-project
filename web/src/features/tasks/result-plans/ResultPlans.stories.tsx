import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, waitFor, within } from 'storybook/test';
import type { ResultPlanView } from '../../../shared/api/contracts';
import { PlanBox } from './PlanBox';
import { PlanRegisterForm } from './PlanRegisterForm';

const PENDING: ResultPlanView = {
  id: 'RP-2',
  content: 'The working endpoint with a sample request',
  condition: 'When the endpoint answers its first request',
  deliveredResultVersion: null,
};
const DELIVERED: ResultPlanView = { ...PENDING, id: 'RP-1', deliveredResultVersion: 1 };
const FINAL: ResultPlanView = { ...PENDING, id: 'RP-3', final: true };
const JA_PENDING: ResultPlanView = {
  ...PENDING,
  content: '動作するエンドポイントとサンプルリクエスト',
  condition: '最初のリクエストにエンドポイントが応答したら',
};

const PHONE_DARK_JA = { width: 'phone-375', mode: 'dark', locale: 'ja' } as const;

const meta = {
  title: 'Task Detail/PlanBox',
  component: PlanBox,
  args: { plan: PENDING, canRegister: true, onRegister: fn(async () => {}) },
} satisfies Meta<typeof PlanBox>;

export default meta;
type Story = StoryObj<typeof meta>;

/** An undelivered box is empty, says so, and offers to register the real result. */
export const Undelivered: Story = {
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(view.getByText('Not delivered')).toBeVisible();
    await userEvent.click(view.getByRole('button', { name: 'Register the result' }));
    await expect(view.getByRole('form', { name: 'Register the result of RP-2' })).toBeVisible();
  },
};

export const UndeliveredJapanese: Story = { args: { plan: JA_PENDING }, globals: { locale: 'ja' } };

/** A delivered box holds the result that delivered it. */
export const Delivered: Story = {
  args: { plan: DELIVERED, children: <p>Result 1 is shown here.</p> },
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(view.getByText('Delivered')).toBeVisible();
    await expect(view.queryByRole('button', { name: 'Register the result' })).toBeNull();
  },
};

export const FinalBox: Story = { args: { plan: FINAL } };

export const ReadOnly: Story = {
  args: { canRegister: false },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).queryByRole('button', { name: 'Register the result' }),
    ).toBeNull();
  },
};

let attempts = 0;

/** The first request fails: the input stays, the reason shows, and Try again sends it as it was. */
export const RegisterRetry: Story = {
  args: {
    onRegister: fn(async () => {
      attempts += 1;
      if (attempts === 1) throw new Error('The server could not be reached.');
    }),
  },
  play: async ({ canvasElement }) => {
    attempts = 0;
    const view = within(canvasElement);
    await userEvent.click(view.getByRole('button', { name: 'Register the result' }));
    await userEvent.type(view.getByLabelText('Summary'), 'Endpoint answers');
    await userEvent.click(view.getByRole('button', { name: 'Register' }));
    await expect(await view.findByRole('alert')).toHaveTextContent(
      'The result could not be registered.',
    );
    await expect(view.getByLabelText('Summary')).toHaveValue('Endpoint answers');
    await userEvent.click(view.getByRole('button', { name: 'Try again' }));
    await waitFor(() => expect(attempts).toBe(2));
    await waitFor(() => expect(view.queryByRole('form')).toBeNull());
  },
};

/** The form: summary, link, and pictures, all optional (a result can be text, a URL, or an image). */
export const RegisterForm: Story = {
  render: () => (
    <div style={{ maxWidth: 520 }}>
      <PlanRegisterForm
        planId="RP-2"
        placeholder={PENDING.content}
        onSubmit={async () => {}}
        onCancel={() => {}}
      />
    </div>
  ),
};

export const RegisterFormPhone: Story = {
  render: RegisterForm.render,
  globals: PHONE_DARK_JA,
};
