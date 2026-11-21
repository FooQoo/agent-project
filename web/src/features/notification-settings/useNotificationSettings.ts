import { useCallback, useEffect, useRef, useState } from 'react';
import { queryKeys } from '../../shared/data/core/query-keys';
import { useReadQuery } from '../../shared/data/react';
import { loadNotificationSettings, saveNotificationSettings } from './api';
import type { NotificationSettings, NotificationSettingsPatch } from './model';

export type NotificationSettingsState =
  | { status: 'loading' }
  | { status: 'failed' }
  | { status: 'ready'; settings: NotificationSettings; saveFailed: boolean };

/**
 * Reads the saved settings when the section appears, and saves each change. A change is shown at
 * once; saves go out one at a time in the order they were made, and a failed save goes back to
 * the last saved value and says so.
 */
export function useNotificationSettings(
  load: () => Promise<NotificationSettings> = loadNotificationSettings,
  save: (patch: NotificationSettingsPatch) => Promise<NotificationSettings> = (patch) =>
    saveNotificationSettings(patch),
) {
  const [state, setState] = useState<NotificationSettingsState>({ status: 'loading' });
  const saved = useRef<NotificationSettings | null>(null);
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const pending = useRef(0);
  const alive = useRef(true);

  const query = useReadQuery({ queryKey: queryKeys.notificationSettings(), queryFn: load });
  const { data, isError, dataUpdatedAt } = query;
  // biome-ignore lint/correctness/useExhaustiveDependencies: every finished read applies, changed or not.
  useEffect(() => {
    if (data) {
      saved.current = data;
      setState({ status: 'ready', settings: data, saveFailed: false });
    } else if (isError) setState({ status: 'failed' });
  }, [data, isError, dataUpdatedAt]);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const change = useCallback(
    (patch: NotificationSettingsPatch) => {
      setState((current) =>
        current.status === 'ready'
          ? { status: 'ready', settings: { ...current.settings, ...patch }, saveFailed: false }
          : current,
      );
      pending.current += 1;
      queue.current = queue.current.then(() =>
        save(patch).then(
          (settings) => {
            pending.current -= 1;
            saved.current = settings;
            if (alive.current && pending.current === 0) {
              setState({ status: 'ready', settings, saveFailed: false });
            }
          },
          () => {
            pending.current -= 1;
            const last = saved.current;
            if (alive.current && last && pending.current === 0) {
              setState({ status: 'ready', settings: last, saveFailed: true });
            }
          },
        ),
      );
    },
    [save],
  );

  return { state, change };
}
