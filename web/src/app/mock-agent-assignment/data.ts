import type { AcceptanceItem, EpicRef, TaskSummary } from '../../shared/api/contracts';

/** The four lanes of the board. Backlog is the stored lane before Todo. */
export type Lane = 'backlog' | 'todo' | 'doing' | 'done';
export const LANES: readonly Lane[] = ['backlog', 'todo', 'doing', 'done'];

export const PROJECT_ID = '0f1e2d3c4b5a69788796a5b4c3d2e1f0';
const NOW = '2026-11-24T09:00:00.000Z';

/** The state of the parent session an agent is tied to (SPEC 20_AGENT_ASSIGNMENT, 4 and 6.1). */
export type ConnectionState = 'connected' | 'unconnected' | 'replaced';

export interface MockSession {
  tool: string;
  label: string;
  /** Minutes since the last activity. */
  idleMin: number;
}

export interface MockAgent {
  /** `A-<n>`, never reused. Cards point at this, so a rename keeps them. */
  id: string;
  name: string;
  role: string;
  color: string;
  state: ConnectionState;
  /** The session that works as this agent now (state connected). */
  session: MockSession | null;
  /** A session this agent had before a switch; its updates are rejected. */
  replacedSession: MockSession | null;
}

export interface MockCard {
  id: number;
  title: string;
  epicId: string | null;
  lane: Lane;
  /** Every AI card has an agent (the default one unless changed); a human card has none. */
  agentId: string | null;
  /** A card for a person: no agent, no avatar. */
  human?: boolean;
  /** Raised by every claim, assignment and handover (SPEC 6.5). */
  generation: number;
  running: boolean;
  body: string;
  conditions: AcceptanceItem[];
  /** The text of an open question, if the card waits on one. */
  question: string | null;
  /** Numbers shown in the handover dialog ("what carries over"). */
  results: number;
  history: string[];
  /** Assigned in the last operation. */
  fresh?: boolean;
}

export interface Fixture {
  epics: EpicRef[];
  epicBody: Record<string, string>;
  agents: MockAgent[];
  cards: MockCard[];
}

const cond = (id: string, text: string, done = false): AcceptanceItem => ({ id, text, done });

export function buildFixture(ja: boolean): Fixture {
  const t = (jaText: string, enText: string) => (ja ? jaText : enText);
  const login: EpicRef = {
    id: 'LOGIN-1',
    name: t('ログインの見直し', 'Rework sign-in'),
    color: '#3b82f6',
  };
  const search: EpicRef = {
    id: 'SEARCH-2',
    name: t('検索の改善', 'Improve search'),
    color: '#10b981',
  };
  const agents: MockAgent[] = [
    {
      id: 'A-1',
      name: t('エージェント', 'Agent'),
      role: t(
        '既定の担当。AI のカードはすべてここに付きます',
        'The default agent: every AI card starts here',
      ),
      color: '#2563eb',
      state: 'connected',
      session: { tool: 'Claude Code', label: 'ui-work', idleMin: 2 },
      replacedSession: { tool: 'Claude Code', label: 'ui-work-old', idleMin: 95 },
    },
    {
      id: 'A-2',
      name: t('CLI担当', 'CLI agent'),
      role: t('CLI とサーバーの実装', 'CLI and server code'),
      color: '#c2410c',
      state: 'connected',
      session: { tool: 'Codex', label: 'cli-work', idleMin: 0 },
      replacedSession: null,
    },
    {
      id: 'A-3',
      name: t('レビュー担当', 'Review agent'),
      role: t('PR のレビューと検証', 'Pull request review and checks'),
      color: '#7c3aed',
      state: 'unconnected',
      session: null,
      replacedSession: null,
    },
    {
      id: 'A-4',
      name: t('調査担当', 'Research agent'),
      role: t('仕様と先行事例の調査', 'Specs and prior art'),
      color: '#0f766e',
      state: 'replaced',
      session: null,
      replacedSession: { tool: 'Claude Code', label: 'research', idleMin: 41 },
    },
  ];
  const mk = (
    id: number,
    title: string,
    epicId: string | null,
    lane: Lane,
    agentId: string | null,
    extra: Partial<MockCard> = {},
  ): MockCard => ({
    id,
    title,
    epicId,
    lane,
    agentId,
    generation: agentId ? 2 : 1,
    running: false,
    body: t(
      `${title}。\n\n## 進め方\n\n1. 現状を確かめる\n2. 変更して確かめる`,
      `${title}.\n\n## Steps\n\n1. Check the current state\n2. Change it and verify`,
    ),
    conditions: [
      cond(
        'AC-1',
        t('変更の対象が一覧になっている', 'The targets of the change are listed'),
        lane === 'done',
      ),
      cond('AC-2', t('動作が確かめられている', 'The behavior is verified'), lane === 'done'),
    ],
    question: null,
    results: 0,
    history: [t('カードを作成しました。', 'Created the card.')],
    ...extra,
  });
  const cards: MockCard[] = [
    mk(
      41,
      t('現行のログイン処理を調べる', 'Survey the current sign-in flow'),
      'LOGIN-1',
      'backlog',
      'A-1',
    ),
    mk(42, t('認証方式の候補を比べる', 'Compare sign-in methods'), 'LOGIN-1', 'backlog', 'A-1', {
      question: t('SSO は今回の対象に含めますか？', 'Is SSO in scope this time?'),
    }),
    mk(
      43,
      t('選んだ方式でログイン画面を作る', 'Build the sign-in screen'),
      'LOGIN-1',
      'backlog',
      'A-1',
    ),
    mk(
      44,
      t('既存利用者の移行手順を書く', 'Write the migration steps'),
      'LOGIN-1',
      'backlog',
      'A-1',
    ),
    mk(45, t('ログインのエラー文言を直す', 'Reword the sign-in errors'), 'LOGIN-1', 'todo', 'A-1'),
    mk(
      46,
      t('パスワード再設定の経路を作る', 'Build the password reset path'),
      'LOGIN-1',
      'doing',
      'A-2',
      {
        running: true,
        generation: 3,
        results: 2,
        history: [
          t('A-1 から A-2 へ担当を替えました。', 'Handed over from A-1 to A-2.'),
          t('着手しました。', 'Started work.'),
        ],
      },
    ),
    mk(
      47,
      t('ログインの現状を洗い出す', 'List the current sign-in problems'),
      'LOGIN-1',
      'done',
      'A-3',
    ),
    mk(
      31,
      t('検索結果の並び順を直す', 'Fix the order of search results'),
      'SEARCH-2',
      'todo',
      null,
      { human: true },
    ),
    mk(
      28,
      t('検索のインデックスを作り直す', 'Rebuild the search index'),
      'SEARCH-2',
      'doing',
      'A-2',
      {
        running: true,
        generation: 2,
        results: 3,
        question: null,
        history: [
          t('インデックスの作り直しを始めました。', 'Started the index rebuild.'),
          t(
            '質問に回答が付きました（未受け取り）。',
            'A question was answered (not yet received).',
          ),
        ],
      },
    ),
    mk(
      27,
      t('検索画面の空表示を作る', 'Build the empty state of search'),
      'SEARCH-2',
      'doing',
      'A-1',
      {
        running: true,
        results: 1,
      },
    ),
    mk(20, t('検索の現状を調べる', 'Survey the current search'), 'SEARCH-2', 'done', 'A-3'),
  ];
  return {
    epics: [login, search],
    epicBody: {
      'LOGIN-1': `# ${login.name}\n\n${t('社内ツールのログインを、迷わず入れる形にそろえる。', 'Make sign-in for the internal tool one that people can get through without help.')}`,
      'SEARCH-2': `# ${search.name}\n\n${t('検索を速く、見つけやすくする。', 'Make search faster and easier to read.')}`,
    },
    agents,
    cards,
  };
}

export const agentOf = (agents: readonly MockAgent[], id: string | null) =>
  agents.find((agent) => agent.id === id) ?? null;

export const countIn = (cards: readonly MockCard[], lane: Lane) =>
  cards.filter((card) => card.lane === lane).length;

/** The cards an agent holds that are not finished. */
export const loadOf = (cards: readonly MockCard[], agentId: string) =>
  cards.filter((card) => card.agentId === agentId && card.lane !== 'done').length;

export function summaryOf(card: MockCard, epics: readonly EpicRef[]): TaskSummary {
  return {
    id: card.id,
    title: card.title,
    assignee: card.human ? 'human' : 'ai',
    taskStatus: card.lane === 'backlog' ? 'todo' : card.lane,
    taskStatusChangedAt: NOW,
    position: card.id,
    agentStatus: card.running ? 'running' : 'idle',
    agentStatusChangedAt: NOW,
    pendingReason: null,
    pendingRequestId: null,
    pendingChoices: [],
    awaitingAnswer: false,
    epic: epics.find((item) => item.id === card.epicId) ?? null,
    updatedAt: NOW,
    draft: null,
    acceptance: card.conditions,
  };
}
