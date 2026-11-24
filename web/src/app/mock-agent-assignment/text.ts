import { useTranslation } from 'react-i18next';

/** The strings this mock adds. The shared locale files stay untouched. */
const ja = {
  backlog: 'バックログ',
  todo: '未着手',
  doing: '進行中',
  done: '完了',
  laneEmpty: 'カードはありません',
  close: '閉じる',
  cancel: 'キャンセル',
  // mode slot (owned by INTERACTION-MODES; the value is a placeholder)
  modeLabel: 'モード',
  modeValue: '組手',
  // strip
  agentMeta: (id: string, n: number) => `ID ${id} ・ 担当 ${n} 枚`,
  // connection states
  stateConnected: '接続中',
  stateUnconnected: '未接続',
  sessionOf: (tool: string, label: string) => `${tool} ・ ${label}`,
  idle: (n: number) => (n === 0 ? 'たった今' : `${n} 分前`),
  noSession: '親セッションがありません',
  // agents dialog
  agentsTitle: '担当',
  agentsLead:
    '名前と役割で担当を分け、それぞれに親セッションを一つ結び付けます。カードは担当 ID で結ばれるので、改名しても保たれます。',
  agentAdd: '担当を登録',
  agentEdit: '編集',
  agentCode: '接続コードを出す',
  agentSwitch: '接続を切り替える',
  agentPcOnly: '接続コードは PC の画面でだけ出せます',
  agentName: '名前',
  agentNameHint: '1〜50 字。プロジェクト内で重複しません',
  agentRole: '役割',
  agentRoleHint: '〜100 字。一言で書きます',
  agentSave: '登録する',
  // switch dialog
  switchTitle: '接続を切り替える',
  switchTarget: '切り替える担当',
  switchNow: '今の親セッション',
  switchAfter: '切り替え後',
  switchAfterBody: 'コードを入力した新しい親セッションが、この担当を動かします',
  switchEffect1:
    '今の親セッションは、新しいセッションが接続した時点で置き換えられ、以後の更新は拒否されます。',
  switchEffect2: (n: number) =>
    `稼働中の ${n} 枚は、新しいセッションが引き継ぐまで待機に戻ります。`,
  switchEffect3: 'カードの担当と履歴は変わりません。',
  switchShowCode: '切り替えコードを出す',
  switchCodeTitle: '切り替えコード',
  switchCodeBody:
    '新しい親セッションで次のコマンドを実行してください。コードは一度だけ使え、10 分で失効します。',
  switchCodeWait: '新しいセッションの接続を待っています。それまで今のセッションは有効です。',
  switchCodeExpires: '残り 9:42',
  // card detail
  assignChange: '担当を替える',
  // reassign dialog
  reassignTitle: '担当を替える',
  // bulk dialog
  // results
  resultReassigned: (id: number, from: string, to: string) =>
    `#${id} の担当を${from}から${to}に替えました`,
};

const en: typeof ja = {
  backlog: 'Backlog',
  todo: 'Todo',
  doing: 'Doing',
  done: 'Done',
  laneEmpty: 'No cards',
  close: 'Close',
  cancel: 'Cancel',
  modeLabel: 'Mode',
  modeValue: 'Kumite',
  agentMeta: (id, n) => `ID ${id} ・ ${n} cards`,
  stateConnected: 'Connected',
  stateUnconnected: 'Not connected',
  sessionOf: (tool, label) => `${tool} · ${label}`,
  idle: (n) => (n === 0 ? 'just now' : `${n} min ago`),
  noSession: 'No parent session',
  agentsTitle: 'Agents',
  agentsLead:
    'Split work by name and role, and tie one parent session to each. Cards point at the agent ID, so a rename keeps them.',
  agentAdd: 'Register an agent',
  agentEdit: 'Edit',
  agentCode: 'Show connection code',
  agentSwitch: 'Switch connection',
  agentPcOnly: 'Connection codes are shown only on the PC screen',
  agentName: 'Name',
  agentNameHint: '1 to 50 characters, unique in the project',
  agentRole: 'Role',
  agentRoleHint: 'Up to 100 characters, in one line',
  agentSave: 'Register',
  switchTitle: 'Switch connection',
  switchTarget: 'Agent to switch',
  switchNow: 'Parent session now',
  switchAfter: 'After the switch',
  switchAfterBody: 'The new parent session that enters the code will run this agent',
  switchEffect1:
    'The current parent session is replaced when the new one connects, and its later updates are rejected.',
  switchEffect2: (n) =>
    `${n} running card${n === 1 ? '' : 's'} go back to waiting until the new session takes over.`,
  switchEffect3: 'The cards keep their agent and their history.',
  switchShowCode: 'Show the switch code',
  switchCodeTitle: 'Switch code',
  switchCodeBody:
    'Run this command in the new parent session. The code works once and expires in 10 minutes.',
  switchCodeWait: 'Waiting for the new session to connect. The current one stays valid until then.',
  switchCodeExpires: '9:42 left',
  assignChange: 'Change agent',
  reassignTitle: 'Change agent',
  resultReassigned: (id, from, to) => `Handed #${id} over from ${from} to ${to}`,
};

export type MockText = typeof ja;

/** The mock's strings for the language of the Story. */
export function useMockText(): MockText {
  const { i18n } = useTranslation();
  return i18n.language.startsWith('ja') ? ja : en;
}

/** The language of the Story, for fixture text that differs by language. */
export function useIsJa(): boolean {
  const { i18n } = useTranslation();
  return i18n.language.startsWith('ja');
}
