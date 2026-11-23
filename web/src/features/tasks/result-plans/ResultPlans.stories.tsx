import type { Meta, StoryObj } from '@storybook/react-vite';
import { useState } from 'react';
import { expect, fn, userEvent, waitFor, within } from 'storybook/test';
import type { ResultPlanView } from '../../../shared/api/contracts';
import { PlanRegisterForm } from './PlanRegisterForm';
import { ResultPlansDock } from './ResultPlansDock';
import { ResultPlansEditor } from './ResultPlansEditor';
import { ResultPlansSection } from './ResultPlansSection';
import { itemOf, type ResultPlanItem } from './model/resultPlans';

/** A plan as the stories write it; `delivered` is the version of the result that delivered it. */
function plan(
  id: string,
  content: string,
  condition: string,
  options: { final?: boolean; delivered?: number } = {},
): ResultPlanView {
  return {
    id,
    content,
    condition,
    ...(options.final ? { final: true } : {}),
    deliveredResultVersion: options.delivered ?? null,
  };
}

const LAYOUT = plan('RP-1', 'A screenshot of the new layout', 'Once the layout is fixed');
const ENDPOINT = plan(
  'RP-2',
  'The working endpoint with a sample request',
  'When the endpoint answers its first request',
);
const REPORT = plan(
  'RP-3',
  'The finished report with every figure',
  'When every acceptance condition is met',
  { final: true },
);
const SEVERAL = [LAYOUT, ENDPOINT, REPORT];
const MIXED = [{ ...LAYOUT, deliveredResultVersion: 1 }, ENDPOINT, REPORT];

const PHONE_DARK = { width: 'phone-375', mode: 'dark' } as const;
const PHONE_DARK_JA = { width: 'phone-375', mode: 'dark', locale: 'ja' } as const;

const meta = {
  title: 'Task Detail/ResultPlans',
  component: ResultPlansSection,
  args: {
    plans: SEVERAL,
    canRegister: true,
    onRegister: fn(async () => {}),
    onOpenResult: fn(),
  },
} satisfies Meta<typeof ResultPlansSection>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Nothing delivered yet: every row says so and offers to register the real result. */
export const Undelivered: Story = {
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(view.getAllByText('Not delivered')).toHaveLength(3);
    await userEvent.click(
      (await view.findAllByRole('button', { name: 'Register the result' }))[0] as HTMLElement,
    );
    await expect(view.getByRole('form', { name: 'Register the result of RP-1' })).toBeVisible();
  },
};

/** The same rows on a phone-width window, in the dark theme. */
export const UndeliveredPhone: Story = { globals: PHONE_DARK };

/** A delivered plan names its result and opens it; the others still offer to register. */
export const Delivered: Story = {
  args: { plans: MIXED },
  play: async ({ canvasElement, args }) => {
    const view = within(canvasElement);
    await expect(view.getByText('Delivered')).toBeVisible();
    await userEvent.click(view.getByRole('button', { name: 'Open result 1' }));
    await expect(args.onOpenResult).toHaveBeenCalledWith(1);
  },
};

export const DeliveredPhone: Story = {
  args: { plans: MIXED },
  globals: PHONE_DARK_JA,
};

/** A task whose only plan is the final one. */
export const FinalOnly: Story = {
  args: { plans: [REPORT] },
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(view.getAllByRole('listitem')).toHaveLength(1);
    await expect(view.getByText('Final')).toBeVisible();
  },
};

export const FinalOnlyPhone: Story = { args: { plans: [REPORT] }, globals: PHONE_DARK };

const JA_LAYOUT = plan('RP-1', '新しいレイアウトのスクリーンショット', 'レイアウトが固まったら');
const JA_ENDPOINT = plan(
  'RP-2',
  '動作するエンドポイントとサンプルリクエスト',
  '最初のリクエストにエンドポイントが応答したら',
);
const JA_REPORT = plan('RP-3', '全項目を含む最終レポート', 'すべての条件を満たしたら', {
  final: true,
});

/** The same rows in Japanese, with Japanese plan text. */
export const UndeliveredJapanese: Story = {
  args: { plans: [JA_LAYOUT, JA_ENDPOINT, JA_REPORT] },
  globals: { locale: 'ja' },
};

export const DeliveredJapanese: Story = {
  args: { plans: [{ ...JA_LAYOUT, deliveredResultVersion: 1 }, JA_ENDPOINT, JA_REPORT] },
  globals: { locale: 'ja' },
};

export const FinalOnlyJapanese: Story = {
  args: { plans: [JA_REPORT] },
  globals: { locale: 'ja' },
};

/** A task with no plans draws nothing: the Results tab is what it was. */
export const NoPlans: Story = {
  args: { plans: [] },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole('list')).toBeNull();
  },
};

export const NoPlansPhone: Story = { args: { plans: [] }, globals: PHONE_DARK };

/** Several plans, one of them delivered, in the order they are planned. */
export const SeveralPlans: Story = { args: { plans: MIXED } };

export const SeveralPlansPhone: Story = { args: { plans: MIXED }, globals: PHONE_DARK_JA };

/** A person who cannot record results sees the state and the open button, without the register one. */
export const ReadOnly: Story = {
  args: { plans: MIXED, canRegister: false },
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
    await userEvent.click(
      (await view.findAllByRole('button', { name: 'Register the result' }))[0] as HTMLElement,
    );
    await userEvent.type(view.getByLabelText('Summary'), 'Layout is fixed');
    await userEvent.click(view.getByRole('button', { name: 'Register' }));
    await expect(await view.findByRole('alert')).toHaveTextContent(
      'The result could not be registered.',
    );
    await expect(view.getByLabelText('Summary')).toHaveValue('Layout is fixed');
    await userEvent.click(view.getByRole('button', { name: 'Try again' }));
    await waitFor(() => expect(attempts).toBe(2));
    await waitFor(() => expect(view.queryByRole('form')).toBeNull());
  },
};

function EditorState({ initial, open = true }: { initial: ResultPlanItem[]; open?: boolean }) {
  const [items, setItems] = useState(initial);
  return (
    <div style={{ maxWidth: 640 }}>
      <ResultPlansDock items={items} onChange={setItems} defaultOpen={open} />
    </div>
  );
}

const EDITING = SEVERAL.map((item, index) =>
  itemOf(index === 0 ? { ...item, deliveredResultVersion: 1 } : item),
);

/** The edit dialog's dock for the planned results: what, in which state, and whether final. */
export const EditorSeveralPlans: Story = {
  render: () => <EditorState initial={EDITING} />,
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(view.getAllByLabelText('What will be delivered')).toHaveLength(3);
    // A delivered plan cannot be removed.
    await expect(view.getByRole('button', { name: 'Remove RP-1' })).toBeDisabled();
    await userEvent.click(view.getByRole('button', { name: 'Add plan' }));
    await expect(view.getAllByLabelText('What will be delivered')).toHaveLength(4);
  },
};

export const EditorPhone: Story = {
  render: () => <EditorState initial={EDITING} />,
  globals: PHONE_DARK_JA,
};

/** The editor with Japanese plan text. */
export const EditorJapanese: Story = {
  render: () => (
    <EditorState
      initial={[{ ...itemOf(JA_LAYOUT), delivered: 1 }, itemOf(JA_ENDPOINT), itemOf(JA_REPORT)]}
    />
  ),
  globals: { locale: 'ja' },
};

/** Folded to a line with the count, as the dialog opens. */
export const EditorFolded: Story = {
  render: () => <EditorState initial={EDITING} open={false} />,
};

/** A plan with an empty field stops the save and says which one. */
export const EditorProblems: Story = {
  render: () => (
    <div style={{ maxWidth: 640 }}>
      <ResultPlansEditor
        items={[itemOf(LAYOUT), { ...itemOf(REPORT), content: '' }]}
        onChange={() => {}}
      />
    </div>
  ),
  play: async ({ canvasElement }) => {
    await expect(await within(canvasElement).findByRole('alert')).toHaveTextContent(
      'Enter what will be delivered',
    );
  },
};

/** Two final plans stop the save: only one plan can be the final result. */
export const EditorTwoFinals: Story = {
  render: () => (
    <div style={{ maxWidth: 640 }}>
      <ResultPlansEditor
        items={[itemOf(REPORT), itemOf({ ...ENDPOINT, final: true })]}
        onChange={() => {}}
      />
    </div>
  ),
  play: async ({ canvasElement }) => {
    await expect(await within(canvasElement).findByRole('alert')).toHaveTextContent(
      'Only one plan can be the final result.',
    );
  },
};

/** The form of one plan's row: summary, link, and pictures, all optional. */
export const RegisterForm: Story = {
  render: () => (
    <div style={{ maxWidth: 520 }}>
      <PlanRegisterForm
        planId="RP-1"
        placeholder={LAYOUT.content}
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
