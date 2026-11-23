import type { HeaderSettings } from '../../shared/ui/app-header/Header';
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ProjectId } from '../../shared/api/contracts';
import { httpStatusOf } from '../../shared/api/httpStatus';
import { DataProvider, usePresentationHold, useSummary } from '../../shared/data/react';
import type { ConnectionState, DataAdapter } from '../../shared/data/core';
import {
  applyPreferenceEvent,
  syncPreferences,
  type PreferencesEndpoint,
} from '../../shared/preferences';
import type { ResultLinkTarget } from '../../features/tasks/results/resultLink';
import type { MessagesApi } from '../../features/tasks/messages/model/messages';
import type { PhoneGuideLink } from '../../features/onboarding/model/phoneGuideLink';
import type {
  MonitorHeaderProps,
  MonitorReadResolvers,
  MonitorViewProps,
  MonitorWrites,
} from './contracts';
import { AskAiSheet } from '../../features/messages/ask/AskAi';
import { MonitorView } from './View';
import { monitorFreshness } from './model/freshness';
import { filterByVersion } from '../../features/versions/model/versionFilter';
import { useVersionFilter } from '../../features/versions/model/useVersionFilter';
import { HeaderReleaseVersion } from '../../features/versions/ui/HeaderReleaseVersion';
import { selectMonitorTasks } from './model/select';
import { monitorTasksOf } from './model/summary';
import { planProject } from './model/planProject';
import { PlanContext } from '../../features/versions/ui/PlanContext';

/** A hook that reloads the page for a newer build (`served`), unless the page is `busy`. The entry supplies it. */
export type BuildReloadHook = (served: string | null, busy: boolean) => void;

/**
 * What the screen takes from the app that shows it: where attachments are read and which
 * acceptance condition write it uses, where the language and theme are saved, and how a question
 * is read for the answer sheet. The monitor app and the board app each give their own.
 */
export interface MonitorEnvironment {
  readResolvers: MonitorReadResolvers;
  /** Where the language and theme are saved; `undefined` keeps them in this browser only. */
  preferences?: PreferencesEndpoint;
  /** Reads, answers and withdraws messages for the list of the band. */
  messagesApi: MessagesApi;
  /** Moves and archives from the lanes; without them the lanes only read. */
  writes?: MonitorWrites;
  /** The header's settings dialog; without it, the plain one that saves through `preferences`. */
  settings?: HeaderSettings;
  /** Show the planned version on each card (read only). The board app turns it on. */
  planning?: boolean;
}

export interface MonitorScreenProps extends MonitorEnvironment {
  projectId: ProjectId;
  useBuildReload: BuildReloadHook;
  /** The build the server named; `null` when the app reloads for a new build itself. */
  build: string | null;
  stream: ConnectionState;
  /** The project is gone: the app replaces the screen. */
  onNotFound(): void;
  /** The header's project switcher, when the app offers one. */
  header?: MonitorHeaderProps;
  /** The first-run guide's link to this screen, when the app runs the guide. */
  guideLink?: PhoneGuideLink;
  /** A display-only indicator for the status line at the bottom. */
  statusSlot?: ReactNode;
  /** The result a link points at: the drawer opens on it once its task is listed. */
  resultLink?: ResultLinkTarget | null;
  /** The drawer of the link's task was closed: the app takes the link out of the address. */
  onResultLinkClose?: () => void;
  /** The address asks for the messages on the Summary tab; the app takes it out when the list closes. */
  summaryLink?: boolean;
  onSummaryLinkClose?: () => void;
}

export interface MonitorProjectProps extends MonitorEnvironment {
  projectId: ProjectId;
  /** How this project is read. Stable for the life of the screen. */
  adapter: DataAdapter;
  useBuildReload: BuildReloadHook;
  /** The project is gone: the screen is replaced by the error screen, which also ends the stream. */
  onNotFound(): void;
}

/**
 * One project on the phone, in the monitor app. It brings its own data layer; the board app draws `MonitorScreen` inside its own.
 */
export function MonitorProject(props: MonitorProjectProps) {
  const [build, setBuild] = useState<string | null>(null);
  const [stream, setStream] = useState<ConnectionState>('connecting');
  const { preferences } = props;
  const onReady = useCallback(
    (served: string) => {
      setBuild(served);
      // The saved language and theme come from the PC each time the stream is ready.
      if (preferences) void syncPreferences(preferences);
    },
    [preferences],
  );
  const onPreferences = useCallback((event: unknown) => applyPreferenceEvent(event as Event), []);
  return (
    <DataProvider
      adapter={props.adapter}
      onReady={onReady}
      onPreferences={onPreferences}
      onConnectionState={setStream}
    >
      <MonitorScreen {...props} build={build} stream={stream} />
    </DataProvider>
  );
}

/**
 * The phone screen of one project, drawn from the Summary the data layer above it holds, with the
 * Details a person opened.
 */
export function MonitorScreen({
  projectId,
  messagesApi,
  readResolvers,
  preferences,
  useBuildReload,
  onNotFound,
  build,
  stream,
  header,
  guideLink,
  statusSlot,
  writes,
  settings,
  planning = false,
  resultLink,
  onResultLinkClose,
  summaryLink,
  onSummaryLinkClose,
}: MonitorScreenProps) {
  const summary = useSummary(projectId);
  const [detailOpen, setDetailOpen] = useState(false);
  const [messagesOpen, setMessagesOpen] = useState(false);
  // The phone band's "Ask AI" sheet.
  const [asking, setAsking] = useState(false);
  const [requestSent, setRequestSent] = useState(0);
  const askGuard = useRef<((after: () => void) => void) | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [lastFetchedAt, setLastFetchedAt] = useState<string | null>(null);

  // A drawer keeps the list as it is; the data layer applies what arrived 3 seconds after it is
  // gone. A list of messages that is only open holds nothing: the message is replaced or closed
  // in it as it happens. Reloading for a new build waits for either, since a reload would lose an
  // answer being written.
  usePresentationHold(detailOpen, 'drawer');
  useBuildReload(build, detailOpen || messagesOpen || asking);

  const gone = httpStatusOf(summary.error) === 404;
  useEffect(() => {
    if (gone) onNotFound();
  }, [gone, onNotFound]);

  const data = summary.summary;
  useEffect(() => {
    if (data !== null) setLastFetchedAt(new Date().toISOString());
  }, [data]);

  const allTasks = useMemo(() => (data === null ? [] : monitorTasksOf(data)), [data]);
  // The version filter is this screen's own view, as on the PC board: it starts as the release
  // version, keeps the unplanned cards, and never filters the Backlog.
  const { filter, setFilter } = useVersionFilter(projectId, data?.project.releaseVersion ?? null);
  const tasks = useMemo(() => {
    const kept = new Set(
      filterByVersion(
        allTasks.map((entry) => entry.task),
        filter,
      ),
    );
    return allTasks.filter((entry) => kept.has(entry.task));
  }, [allTasks, filter]);
  const freshness = monitorFreshness({
    state: summary.state,
    hasData: data !== null,
    problems: data?.problems ?? [],
    projectId,
    stream,
    lastFetchedAt,
  });
  // Kept between renders: the lanes' overlay tells a newer read from the one it saw by identity, so a
  // render that read nothing new must hand it the same arrays.
  const visible = useMemo(() => selectMonitorTasks(tasks), [tasks]);
  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await summary.refresh();
    } finally {
      setRefreshing(false);
    }
  }, [summary.refresh]);
  const props: MonitorViewProps = {
    summary:
      data === null || freshness.state === 'partial'
        ? null
        : {
            backlog: visible.backlog.length,
            todo: visible.todo.length,
            doing: visible.doing.length,
            done: visible.done.length,
          },
    ...visible,
    projectId,
    messages: data?.messages ?? undefined,
    messagesApi,
    // Answers are shown at once: the Summary is read again without waiting for the hold.
    onMessagesChanged: () => void summary.refresh(),
    onAsk: () => setAsking(true),
    onAskClose: (then) => (askGuard.current ? askGuard.current(then) : then()),
    asking,
    requestSent,
    readResolvers,
    preferences,
    settings,
    header: header && {
      ...header,
      versionSlot: (
        <HeaderReleaseVersion projectId={projectId} filter={filter} onFilter={setFilter} phone />
      ),
    },
    guideLink,
    statusSlot,
    resultLink,
    onResultLinkClose,
    summaryLink,
    onSummaryLinkClose,
    freshness,
    onRefresh: () => void refresh(),
    refreshing,
    onDetailOpenChange: setDetailOpen,
    onMessagesOpenChange: setMessagesOpen,
    writes,
    projectName: header?.projects?.find((project) => project.id === projectId)?.name ?? '',
    onWritten: () => void summary.refresh(),
  };
  return (
    <>
      <PlanContext.Provider value={planProject(planning, data)}>
        <MonitorView {...props} />
      </PlanContext.Provider>
      {asking && (
        <AskAiSheet
          projectId={projectId}
          tasks={allTasks.map((entry) => entry.task)}
          closeGuard={askGuard}
          onClose={() => setAsking(false)}
          onSent={() => {
            setRequestSent((count) => count + 1);
            void summary.refresh();
          }}
        />
      )}
    </>
  );
}
