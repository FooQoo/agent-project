import { useCallback } from 'react';
import type { ProjectId } from '@/shared/api/contracts';
import { queryKeys } from '@/shared/data/core/query-keys';
import { useReadQuery } from '@/shared/data/react';
import type { MessageDetail, MessagesApi } from './model/messages';

export interface MessageDetailState {
  detail: MessageDetail | null;
  state: 'loading' | 'ready' | 'failed';
  reload: () => void;
}

/** Reads the message that is opened (none while the list shows), and reads it again on `reload`. */
export function useMessageDetail(
  read: MessagesApi['read'],
  projectId: ProjectId,
  messageId: string | null,
): MessageDetailState {
  const query = useReadQuery({
    queryKey: queryKeys.message(projectId, messageId ?? ''),
    queryFn: () => read(projectId, messageId as string),
    enabled: messageId !== null,
  });
  const { refetch } = query;
  const reload = useCallback(() => void refetch({ cancelRefetch: false }), [refetch]);
  const detail = messageId !== null ? (query.data ?? null) : null;
  return {
    detail,
    state: detail !== null ? 'ready' : query.isError ? 'failed' : 'loading',
    reload,
  };
}
