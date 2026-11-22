import { useCallback, useRef, useState } from 'react';
import type { PendingSave } from './planField';

/**
 * Holds the change a planned-version field reported (`onPending`), for the editor that saves it as
 * the last step. The change is saved by its own request, so it never makes a card or epic a draft.
 */
export function usePendingPlan() {
  const save = useRef<PendingSave | null>(null);
  const [pending, setPending] = useState(false);
  const onPending = useCallback((next: PendingSave | null) => {
    save.current = next;
    setPending(next !== null);
  }, []);
  return { save, pending, onPending };
}
