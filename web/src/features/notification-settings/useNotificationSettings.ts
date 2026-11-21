import { useCallback, useEffect, useRef, useState } from 'react';
import { queryKeys } from '../../shared/data/core/query-keys';
import { useReadClient, useReadQuery } from '../../shared/data/react';
import { useWrite } from '../../shared/data/react/use-write';
import { loadNotificationSettings, saveNotificationSettings } from './api';
import type { NotificationSettings, NotificationSettingsPatch } from './model';

const WRITE_KEY = ['notification-settings-save'] as const;

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
  const alive = useRef(true);
  const client = useReadClient();

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

  // The saves of one scope run one at a time, in the order they were made. A save that finishes
  // while another is still waiting shows nothing yet; the last one decides what is shown.
  const lastOne = () => client.isMutating({ mutationKey: WRITE_KEY }) <= 1;
  const write = useWrite<NotificationSettings, NotificationSettingsPatch>({
    mutationKey: WRITE_KEY,
    scope: { id: WRITE_KEY[0] },
    mutationFn: (patch) => save(patch),
    onSuccess: (settings) => {
      saved.current = settings;
      if (alive.current && lastOne()) setState({ status: 'ready', settings, saveFailed: false });
    },
    onError: () => {
      const last = saved.current;
      if (alive.current && last && lastOne())
        setState({ status: 'ready', settings: last, saveFailed: true });
    },
  });
  const { mutate } = write;

  const change = useCallback(
    (patch: NotificationSettingsPatch) => {
      setState((current) =>
        current.status === 'ready'
          ? { status: 'ready', settings: { ...current.settings, ...patch }, saveFailed: false }
          : current,
      );
      mutate(patch);
    },
    [mutate],
  );

  return { state, change };
}
