import { useCallback } from 'react';
// A write of the web UI (a save, a move, a decision) as one mutation of the query client. The
// caller gives what the write does and what follows it: `onMutate` shows an optimistic change,
// `onError` takes it back and says why, `onSettled` asks for the data the write changed again.
// A write is never tried again on its own, so a person's Retry stays the only retry. Where there
// is no data provider (a Story, a test of one part) the write uses a client of its own.
import {
  type UseMutationOptions,
  type UseMutationResult,
  useMutation,
} from '@tanstack/react-query';
import { useReadClient } from './use-read-query';

export type WriteOptions<Result, Variables, Context = unknown> = Omit<
  UseMutationOptions<Result, Error, Variables, Context>,
  'retry'
>;

export type Write<Result, Variables, Context = unknown> = UseMutationResult<
  Result,
  Error,
  Variables,
  Context
> & {
  /**
   * Whether a write of this hook whose variables match is on its way. It answers at once, so a
   * second submit in the same event is seen (a render would come too late).
   */
  isWriting(match?: (variables: Variables) => boolean): boolean;
};

export function useWrite<Result, Variables = void, Context = unknown>(
  options: WriteOptions<Result, Variables, Context>,
): Write<Result, Variables, Context> {
  const client = useReadClient();
  const mutation = useMutation<Result, Error, Variables, Context>(
    { ...options, retry: false },
    client,
  );
  const key = options.mutationKey;
  const isWriting = useCallback(
    (match?: (variables: Variables) => boolean) =>
      client.isMutating({
        mutationKey: key,
        predicate: (mutation) =>
          match === undefined || match(mutation.state.variables as Variables),
      }) > 0,
    [client, key],
  );
  return { ...mutation, isWriting };
}
