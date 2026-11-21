import type { EpicRef, TaskSummary } from '../../shared/api/contracts';

/** The versions this project has plans for, oldest first. */
export const VERSIONS = ['v1.8.0', 'v1.9.0', 'v2.0.0'] as const;

/** The current version of the project in the default fixture. */
export const DEFAULT_CURRENT = 'v1.8.0';

/** A card of the mock: the real summary plus the planned version (`null` is "undecided"). */
export interface MockTask extends TaskSummary {
  planned: string | null;
}

const NOW = '2026-11-22T09:00:00.000Z';

const LOGIN: EpicRef = { id: 'LOGIN-1', name: 'LOGIN-1', color: '#4f7cff' };
const REPORT: EpicRef = { id: 'REPORT-2', name: 'REPORT-2', color: '#c46b2d' };

function card(
  id: number,
  title: string,
  taskStatus: TaskSummary['taskStatus'],
  planned: string | null,
  epic: EpicRef | null,
): MockTask {
  return {
    id,
    title,
    assignee: 'ai',
    taskStatus,
    taskStatusChangedAt: NOW,
    position: id,
    agentStatus: taskStatus === 'doing' ? 'running' : 'idle',
    agentStatusChangedAt: NOW,
    pendingReason: null,
    pendingRequestId: null,
    pendingChoices: [],
    epic,
    updatedAt: NOW,
    draft: null,
    acceptance: { done: taskStatus === 'done' ? 2 : 0, total: 2 },
    planned,
  };
}

const TITLES: Record<'ja' | 'en', Record<number, string>> = {
  ja: {
    11: 'ログイン画面のレイアウトを整える',
    12: 'パスワード再設定メールを送る',
    13: 'セッションの有効期限を延ばす',
    14: '二要素認証の入力欄を追加する',
    15: 'ログイン失敗時の文言を直す',
    16: 'ログイン履歴を一覧にする',
    21: '週次レポートの集計を高速化する',
    22: 'レポートのCSV出力を追加する',
    23: 'グラフの凡例を読みやすくする',
    31: 'ダークテーマの配色を見直す',
    32: 'キーボード操作の一覧を作る',
    33: '設定画面の検索を追加する',
    34: '起動時の読み込みを減らす',
  },
  en: {
    11: 'Tidy up the login screen layout',
    12: 'Send the password reset email',
    13: 'Extend the session lifetime',
    14: 'Add the two-factor input field',
    15: 'Fix the wording of a failed login',
    16: 'List the login history',
    21: 'Speed up the weekly report totals',
    22: 'Add CSV export to the report',
    23: 'Make the chart legend easier to read',
    31: 'Review the dark theme colors',
    32: 'Write the keyboard shortcut list',
    33: 'Add search to the settings screen',
    34: 'Reduce the work done at startup',
  },
};

/** Thirteen cards across the lanes: v1.8.0 x5, v1.9.0 x4, v2.0.0 x0, undecided x4 (v2.0.0 has none on purpose). */
export function buildTasks(ja: boolean): MockTask[] {
  const t = TITLES[ja ? 'ja' : 'en'];
  return [
    card(11, t[11] ?? '', 'todo', 'v1.8.0', LOGIN),
    card(12, t[12] ?? '', 'todo', 'v1.9.0', LOGIN),
    card(13, t[13] ?? '', 'todo', null, LOGIN),
    card(14, t[14] ?? '', 'todo', null, LOGIN),
    card(21, t[21] ?? '', 'todo', 'v1.8.0', REPORT),
    card(31, t[31] ?? '', 'todo', 'v1.9.0', null),
    card(15, t[15] ?? '', 'doing', 'v1.8.0', LOGIN),
    card(16, t[16] ?? '', 'doing', 'v1.9.0', LOGIN),
    card(22, t[22] ?? '', 'doing', 'v1.8.0', REPORT),
    card(32, t[32] ?? '', 'doing', null, null),
    card(23, t[23] ?? '', 'done', 'v1.8.0', REPORT),
    card(33, t[33] ?? '', 'done', 'v1.9.0', null),
    card(34, t[34] ?? '', 'done', null, null),
  ];
}

export type VersionFilter =
  | { kind: 'all' }
  | { kind: 'current' }
  | { kind: 'version'; version: string }
  | { kind: 'none' };

/** Whether a task passes the filter; "current" with no current version passes everything. */
export function matches(task: MockTask, filter: VersionFilter, current: string | null): boolean {
  switch (filter.kind) {
    case 'all':
      return true;
    case 'current':
      return current === null ? true : task.planned === current;
    case 'version':
      return task.planned === filter.version;
    case 'none':
      return task.planned === null;
  }
}

export function epicTasks(tasks: MockTask[], epicId: string): MockTask[] {
  return tasks.filter((task) => task.epic?.id === epicId);
}

/** The value of the "no version" choice in a radio group. */
export const NONE = '__none__';
