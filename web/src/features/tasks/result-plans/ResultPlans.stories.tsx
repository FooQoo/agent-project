import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import type { ResultPlanView } from '../../../shared/api/contracts';
import { PlanBox } from './PlanBox';

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

const meta = {
  title: 'Task Detail/PlanBox',
  component: PlanBox,
  args: { plan: PENDING },
} satisfies Meta<typeof PlanBox>;

export default meta;
type Story = StoryObj<typeof meta>;

/** An undelivered box is empty and says so; nothing in it is interactive. */
export const Undelivered: Story = {
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(view.getByText('Not delivered')).toBeVisible();
    await expect(view.queryByRole('button')).toBeNull();
  },
};

export const UndeliveredJapanese: Story = { args: { plan: JA_PENDING }, globals: { locale: 'ja' } };

/** A delivered box holds the result that delivered it. */
export const Delivered: Story = {
  args: { plan: DELIVERED, children: <p>Result 1 is shown here.</p> },
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(view.getByText('Delivered')).toBeVisible();
  },
};

export const FinalBox: Story = { args: { plan: FINAL } };
