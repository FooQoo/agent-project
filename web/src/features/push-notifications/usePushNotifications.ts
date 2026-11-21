import { useCallback, useEffect, useState } from 'react';
import { queryKeys } from '../../shared/data/core/query-keys';
import { useReadQuery } from '../../shared/data/react';
import { useWrite } from '../../shared/data/react/use-write';
import { getPushStatus } from './api';
import {
  pushView,
  readSupport,
  type PushPermission,
  type PushStatus,
  type PushSupport,
  type PushView,
} from './state';
import { disablePush, enablePush, hasBrowserSubscription, type EnableResult } from './subscribe';

export interface PushController {
  view: PushView;
  permission: PushPermission;
  support: PushSupport;
  busy: boolean;
  /** The last turn-on or turn-off failed. */
  failed: boolean;
  enable: () => void;
  disable: () => void;
}

function readPermission(): PushPermission {
  return 'Notification' in window ? Notification.permission : 'default';
}

/** The notification row's state, read from the browser and the PC, with its two actions. */
export function usePushNotifications(): PushController {
  const [support] = useState(readSupport);
  const [permission, setPermission] = useState<PushPermission>(readPermission);
  const [failed, setFailed] = useState(false);

  const query = useReadQuery<PushStatus>({
    queryKey: queryKeys.pushStatus(),
    queryFn: async () => {
      const next = await getPushStatus();
      // A subscription kept on the PC is only useful while the browser still holds its own.
      const held = next.subscribed === true ? await hasBrowserSubscription() : false;
      return { ...next, subscribed: held };
    },
  });
  const status = query.isError ? null : (query.data ?? null);
  const publicKey = status?.publicKey ?? null;
  const { refetch } = query;
  const refresh = useCallback(async () => {
    await refetch({ cancelRefetch: false });
  }, [refetch]);
  // The permission is read again each time a read of the status ends.
  useEffect(() => {
    if (query.fetchStatus === 'idle') setPermission(readPermission());
  }, [query.fetchStatus]);

  // A turn-on or turn-off finishes only once the status is read again, so `busy` covers both.
  const enabling = useWrite<EnableResult, string>({
    mutationKey: ['push-enable'],
    mutationFn: (key) => enablePush(key),
    onSuccess: (result) => setFailed(result === 'failed'),
    onSettled: refresh,
  });
  const disabling = useWrite<boolean>({
    mutationKey: ['push-disable'],
    mutationFn: () => disablePush(),
    onSuccess: (done) => setFailed(!done),
    onSettled: refresh,
  });
  const busy = enabling.isPending || disabling.isPending;

  const enable = () => {
    if (publicKey === null || enabling.isWriting() || disabling.isWriting()) return;
    setFailed(false);
    enabling.mutate(publicKey);
  };

  const disable = () => {
    if (enabling.isWriting() || disabling.isWriting()) return;
    setFailed(false);
    disabling.mutate();
  };

  return {
    view: pushView(support, permission, status),
    permission,
    support,
    busy,
    failed,
    enable,
    disable,
  };
}
