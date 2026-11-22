import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { WriteError } from '../../../shared/api/writeErrors';
import { useWrite } from '../../../shared/data/react/use-write';

/**
 * The archive of the cards picked in the Done lane, for the PC board and the phone. The server archives
 * the list in one save, all or none, so a failure says that nothing changed; a 409 says the cards changed
 * while they were picked. A refused archive keeps the selection, so the person can try again.
 */
export function useArchiveSelected({
  write,
  onSettled,
}: {
  write: (ids: number[]) => Promise<unknown>;
  /** The board is read again whether or not the archive went through. */
  onSettled?: () => void;
}) {
  const { t } = useTranslation();
  const [notice, setNotice] = useState<string | null>(null);
  const dismissNotice = useCallback(() => setNotice(null), []);
  const { mutateAsync, isWriting, isPending } = useWrite<unknown, number[]>({
    mutationKey: ['archive-selected'],
    mutationFn: write,
    onSettled: () => onSettled?.(),
  });
  /** Resolves with whether every picked card was archived. */
  const archive = useCallback(
    async (ids: readonly number[]) => {
      if (ids.length === 0 || isWriting()) return false;
      try {
        await mutateAsync([...ids]);
        setNotice(null);
        return true;
      } catch (cause) {
        setNotice(
          t(
            cause instanceof WriteError && cause.status === 409
              ? 'archive.selection.conflict'
              : 'archive.selection.failed',
          ),
        );
        return false;
      }
    },
    [isWriting, mutateAsync, t],
  );
  return { archive, busy: isPending, notice, dismissNotice };
}
