import { useCallback } from 'react';
import type { ColumnName } from '../../shared/api/contracts';
import { useWrite } from '../../shared/data/react/use-write';

interface Move {
  id: number;
  from: ColumnName;
  to: ColumnName;
}

const KEY = ['lane-move'] as const;

/**
 * The move of one card between two lanes, as a person does it with a drag, a swipe tile, or an
 * action of the details. The card is in the target lane at once (`apply`, the mutation's
 * `onMutate`), the server is asked once, and a refusal puts the card back (`revert`) and says why
 * in a notice (`notify`). Either way `onDone` follows, so the screen reads the project again. A
 * snapshot that arrives before the save is answered keeps the card where the person put it,
 * because `apply` stays on top of any read while the save is in flight; once the server accepts
 * it, `settle` lets the first newer read win, whatever lane it shows (an agent may have moved the
 * card on). The optimistic layer itself is `useMovedCards`, which also covers the Backlog moves.
 * It serves the lane moves that need no confirmation; a Backlog card with an open question goes
 * through `useBacklogMove`, which shares this contract.
 */
export function useLaneMove({
  write,
  apply,
  settle,
  revert,
  onDone,
  notify,
  refusal,
}: {
  /** Saves the move. `from` is the status the screen showed, so a card that moved on is refused. */
  write: (id: number, to: ColumnName, from: ColumnName) => Promise<unknown>;
  apply: (id: number, to: ColumnName) => void;
  /** The write was accepted: only a read newer than the save may now replace the target lane. */
  settle: (id: number) => void;
  revert: (id: number) => void;
  onDone: () => void;
  notify: (message: string) => void;
  refusal: (cause: unknown) => string;
}) {
  const mutation = useWrite<unknown, Move>({
    mutationKey: KEY,
    mutationFn: ({ id, from, to }) => write(id, to, from),
    onMutate: ({ id, to }) => apply(id, to),
    onSuccess: (_result, { id }) => settle(id),
    onError: (cause, { id }) => {
      revert(id);
      notify(refusal(cause));
    },
    onSettled: onDone,
  });
  const { mutateAsync, isWriting } = mutation;
  return useCallback(
    async (id: number, from: ColumnName, to: ColumnName) => {
      // A card already on its way is not moved twice.
      if (from === to || isWriting((move) => move.id === id)) return;
      await mutateAsync({ id, from, to }).catch(() => {});
    },
    [mutateAsync, isWriting],
  );
}
