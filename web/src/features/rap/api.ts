import { authHeaders } from '../../shared/api/http';
import type {
  RapChangeRequest,
  RapEvent,
  RapSummary,
  RapWakeupReport,
  RapWorkItem,
  RapWorkKind,
  RapWorkResults,
} from './model';

export class RapApiError extends Error {
  constructor(
    readonly status: number,
    message?: string,
  ) {
    super(message ?? `RAP API request failed (${status})`);
  }
}

async function requestJson<T>(
  url: string,
  init: RequestInit,
  fetcher: typeof fetch = fetch,
): Promise<T> {
  let response: Response;
  try {
    response = await fetcher(url, { cache: 'no-store', ...init });
  } catch {
    throw new RapApiError(0);
  }
  const payload = await response.json().catch(() => null);
  if (!response.ok || payload === null) {
    const detail =
      payload && typeof payload === 'object' && 'error' in payload
        ? String((payload as { error: unknown }).error)
        : undefined;
    throw new RapApiError(response.status, detail);
  }
  return payload as T;
}

function postInit(body: unknown): RequestInit {
  return {
    method: 'POST',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/json',
      ...authHeaders(),
    },
    body: JSON.stringify(body),
  };
}

export function loadRapSummary(
  projectId: string,
  fetcher: typeof fetch = fetch,
): Promise<RapSummary> {
  return requestJson<RapSummary>(
    `/api/rap/projects/${encodeURIComponent(projectId)}/summary`,
    { headers: { Accept: 'application/json' } },
    fetcher,
  );
}

export function createRapWork(
  projectId: string,
  input: {
    workspace: string;
    kind: RapWorkKind;
    parent_epic_id?: number | null;
    title: string;
    rank?: number;
  },
  fetcher: typeof fetch = fetch,
): Promise<RapWorkItem> {
  return requestJson<RapWorkItem>(
    `/api/rap/projects/${encodeURIComponent(projectId)}/work`,
    postInit(input),
    fetcher,
  );
}

export function approveRapPlan(
  workId: number,
  revisionId: number,
  authorizedBy: string,
  fetcher: typeof fetch = fetch,
): Promise<unknown> {
  return requestJson(
    `/api/rap/work/${workId}/approve-plan`,
    postInit({ revision_id: revisionId, authorized_by: authorizedBy }),
    fetcher,
  );
}

export function authorizeRapDirect(
  workId: number,
  brief: string,
  authorizedBy: string,
  fetcher: typeof fetch = fetch,
): Promise<unknown> {
  return requestJson(
    `/api/rap/work/${workId}/authorize-direct`,
    postInit({ brief, authorized_by: authorizedBy }),
    fetcher,
  );
}

export function submitRapChangeRequest(
  workId: number,
  body: string,
  fetcher: typeof fetch = fetch,
): Promise<RapChangeRequest> {
  return requestJson<RapChangeRequest>(
    `/api/rap/work/${workId}/change-requests`,
    postInit({ body }),
    fetcher,
  );
}

export function answerRapQuestion(
  questionId: number,
  answer: string,
  answeredBy: string,
  fetcher: typeof fetch = fetch,
): Promise<unknown> {
  return requestJson(
    `/api/rap/questions/${questionId}/answer`,
    postInit({ answer, answered_by: answeredBy }),
    fetcher,
  );
}

export function loadRapResults(
  workId: number,
  fetcher: typeof fetch = fetch,
): Promise<RapWorkResults> {
  return requestJson<RapWorkResults>(
    `/api/rap/work/${workId}/results`,
    { headers: { Accept: 'application/json' } },
    fetcher,
  );
}

export function reviseRapResult(
  resultId: number,
  body: string,
  fetcher: typeof fetch = fetch,
): Promise<RapChangeRequest> {
  return requestJson<RapChangeRequest>(
    `/api/rap/results/${resultId}/revise`,
    postInit({ body }),
    fetcher,
  );
}

export function enterRapSleep(fetcher: typeof fetch = fetch): Promise<{ sleeping: boolean }> {
  return requestJson('/api/rap/sleep/enter', postInit({}), fetcher);
}

export function wakeRapSleep(fetcher: typeof fetch = fetch): Promise<RapWakeupReport> {
  return requestJson('/api/rap/sleep/wake', postInit({}), fetcher);
}

export function updateRapSettings(
  settings: { auto_run: boolean; max_sessions: number },
  fetcher: typeof fetch = fetch,
): Promise<{ auto_run: boolean; max_sessions: number }> {
  return requestJson(
    '/api/rap/settings',
    {
      method: 'PUT',
      headers: {
        Accept: 'application/json',
        'Content-Type': 'application/json',
        ...authHeaders(),
      },
      body: JSON.stringify(settings),
    },
    fetcher,
  );
}

export interface RapEventStream {
  close(): void;
}

/** Subscribes to post-commit RAP SSE events and invokes `onResync` on reconnect/lag. */
export function subscribeRapEvents(
  onEvent: (event: RapEvent) => void,
  onResync: () => void,
  createSource: (url: string) => EventSource = (url) => new EventSource(url),
): RapEventStream {
  const source = createSource('/api/rap/events');
  const eventTypes = [
    'work_created',
    'plan_approved',
    'direct_authorized',
    'change_request_received',
    'question_answered',
    'sleep_entered',
    'sleep_woken',
  ];
  for (const kind of eventTypes) {
    source.addEventListener(kind, (raw) => {
      const data = (raw as MessageEvent).data;
      try {
        onEvent(JSON.parse(String(data)) as RapEvent);
      } catch {
        onResync();
      }
    });
  }
  source.addEventListener('resync', () => onResync());
  source.onerror = () => onResync();
  return { close: () => source.close() };
}
