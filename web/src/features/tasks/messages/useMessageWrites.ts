import type { ProjectId } from '@/shared/api/contracts';
import { useWrite } from '@/shared/data/react/use-write';
import type { EditorAttachment } from '@/shared/ui/editor/attachmentTypes';
import type { AnswerItem, MessagesApi } from './model/messages';

/**
 * The two writes of an open message: sending an answer and withdrawing a request. Each is one
 * mutation that calls whatever `api` the screen was given. A second press while one is on its
 * way is refused (`answer` and `withdraw` resolve to `false`); a refusal rejects with the error
 * the api threw, for the screen to show.
 */
export function useMessageWrites(api: MessagesApi, projectId: ProjectId) {
  const answerWrite = useWrite<
    unknown,
    { answers: AnswerItem[]; attachments: readonly EditorAttachment[] }
  >({
    mutationKey: ['message-answer', projectId],
    mutationFn: ({ answers, attachments }) => api.answer(projectId, answers, attachments),
  });
  const withdrawWrite = useWrite<unknown, string>({
    mutationKey: ['message-withdraw', projectId],
    mutationFn: (messageId) => api.withdraw(projectId, messageId),
  });
  return {
    busy: answerWrite.isPending || withdrawWrite.isPending,
    async answer(answers: AnswerItem[], attachments: readonly EditorAttachment[]) {
      if (answerWrite.isWriting()) return false;
      await answerWrite.mutateAsync({ answers, attachments });
      return true;
    },
    async withdraw(messageId: string) {
      if (withdrawWrite.isWriting()) return false;
      await withdrawWrite.mutateAsync(messageId);
      return true;
    },
  };
}
