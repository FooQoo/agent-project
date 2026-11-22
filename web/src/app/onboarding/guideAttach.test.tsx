import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import '@/shared/i18n';
import { AskAiDialog } from '../../features/messages/ask/AskAiDialog';
import { sampleAttachTasks, sampleRequest } from './guideSamples';

describe('the guide offers the sample cards in Ask AI', () => {
  afterEach(() => vi.restoreAllMocks());

  it('has three sample cards per language, numbered like real tasks', () => {
    for (const locale of ['en', 'ja'] as const) {
      const tasks = sampleAttachTasks(locale);
      expect(tasks.map((task) => task.id)).toEqual([1, 2, 3]);
      expect(tasks.every((task) => task.title !== '')).toBe(true);
    }
    expect(sampleAttachTasks('ja')[0]?.title).not.toBe(sampleAttachTasks('en')[0]?.title);
  });

  it('attaches a sample card to the request without any network call', async () => {
    const user = userEvent.setup();
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    render(
      <AskAiDialog
        initialText={sampleRequest('en')}
        tasks={sampleAttachTasks('en')}
        onClose={vi.fn()}
        onSubmit={vi.fn()}
      />,
    );
    await user.click(screen.getByRole('button', { name: 'Attach task' }));
    await user.click(await screen.findByRole('checkbox', { name: /Write the summary/ }));
    await user.click(screen.getByRole('button', { name: 'Attach (1)' }));
    await waitFor(() => expect(screen.queryByRole('checkbox')).toBeNull());
    const field = screen.getByRole<HTMLTextAreaElement>('textbox', {
      name: 'What would you like done?',
    });
    expect(field.value).toBe(`${sampleRequest('en')}\n#2`);
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});
