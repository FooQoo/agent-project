import { useTranslation } from 'react-i18next';

/** The strings this mock adds. The shared locale files stay untouched. */
const ja = {
  backlog: 'Backlog',
  todo: 'Todo',
  doing: 'Doing',
  done: 'Done',
  laneEmpty: 'カードはありません',
  close: '閉じる',
  cancel: 'キャンセル',
  dismiss: '閉じる',
  // mode slot (owned by INTERACTION-MODES; the value is a placeholder)
  modeLabel: 'モード',
  modeValue: '組手',
  // strip
  stripLabel: '担当',
  stripManage: '担当の設定',
  unassigned: '未割り当て',
  unassignedCards: (n: number) => `未割り当て ${n} 枚`,
  cardsOf: (n: number) => `${n} 枚`,
  agentMeta: (id: string, n: number) => `ID ${id} ・ 担当 ${n} 枚`,
  epicAssign: '割り当て…',
  epicUnassigned: (name: string, n: number) => `${name}: 未割り当て ${n} 枚`,
  // connection states
  stateConnected: '接続中',
  stateUnconnected: '未接続',
  stateReplaced: '置き換え済み',
  sessionOf: (tool: string, label: string) => `${tool} ・ ${label}`,
  idle: (n: number) => (n === 0 ? 'たった今' : `${n} 分前`),
  noSession: '親セッションがありません',
  replacedNote: '切り替えで置き換えられた旧セッション。更新は拒否されます',
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
  agentNameTaken: 'この名前はすでに使われています',
  previousSession: '直前の接続',
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
  assignSection: '担当',
  assignChange: '担当を替える',
  assignAssign: '担当を割り当てる',
  assignGeneration: (n: number) => `世代 ${n}`,
  assignRunning: '稼働中',
  assignDoneNote: '完了したカードの担当は変えられません',
  // reassign dialog
  reassignTitle: '担当を替える',
  reassignScope: (id: number, title: string) => `#${id} ${title}`,
  reassignFrom: '今の担当',
  reassignTo: '新しい担当',
  reassignCurrent: '今の担当',
  reassignLoad: (n: number) => `進行中 ${n} 枚`,
  reassignCarry: '引き継がれるもの',
  reassignCarryLane: (lane: string) => `レーン（${lane}のまま）`,
  reassignCarryPlan: '計画と受入条件',
  reassignCarryResults: (n: number) => `成果 ${n} 件`,
  reassignCarryQuestion: '未受け取りの質問への回答 1 件',
  reassignStops: '止まるもの',
  reassignStopsBody: (name: string) =>
    `${name}の稼働と、このカードへの書き込み。遅れて届いた更新は拒否されます。`,
  reassignWarnTitle: '新しい担当には親セッションが接続していません',
  reassignWarnBody:
    'カードは今のレーンのまま、接続するまで誰にも動かされません。それでも替えますか。',
  reassignWarnAck: '接続されるまで止まることを確認しました',
  reassignNeedAck: '確認してから替えてください',
  reassignConfirm: (name: string) => `${name}に替える`,
  reassignPick: '新しい担当を選んでください',
  // bulk dialog
  bulkTitle: (epic: string) => `${epic}の未割り当てカードを割り当てる`,
  bulkLead:
    '担当が決まっていないカードだけが対象です。すでに担当のいるカードと完了したカードは変わりません。',
  bulkAgent: '割り当てる担当',
  bulkWill: (n: number) => `割り当てるカード（${n} 枚）`,
  bulkSkip: (n: number) => `変わらないカード（${n} 枚）`,
  bulkSkipAssigned: (name: string) => `${name}が担当済み`,
  bulkSkipDone: '完了済み',
  bulkAfter: (name: string, a: number, b: number) => `${name}の持ち札 ${a} → ${b} 枚`,
  bulkLaneKept: 'レーンは変わりません',
  bulkLater: '後から追加したカードは対象外です',
  bulkConfirm: (n: number, name: string) => `${n} 枚を${name}に割り当てる`,
  bulkNone: '未割り当てのカードはありません',
  // results
  resultReassigned: (id: number, from: string, to: string) =>
    `#${id} の担当を${from}から${to}に替えました`,
  resultReassignedBody: (g0: number, g1: number) =>
    `履歴に残しました ・ 世代 ${g0} → ${g1} ・ 新担当が接続すると続きから始めます`,
  resultBulk: (n: number, name: string) => `${n} 枚を${name}に割り当てました`,
  resultBulkBody: (kept: number) =>
    `変わらなかったカード ${kept} 枚 ・ 割り当てたカードには印を付けています`,
  freshMark: '割り当てたばかり',
  handedMark: '引き継いだばかり',
};

const en: typeof ja = {
  backlog: 'Backlog',
  todo: 'Todo',
  doing: 'Doing',
  done: 'Done',
  laneEmpty: 'No cards',
  close: 'Close',
  cancel: 'Cancel',
  dismiss: 'Dismiss',
  modeLabel: 'Mode',
  modeValue: 'Kumite',
  stripLabel: 'Agents',
  stripManage: 'Manage agents',
  unassigned: 'Unassigned',
  unassignedCards: (n) => `${n} unassigned`,
  cardsOf: (n) => `${n}`,
  agentMeta: (id, n) => `ID ${id} ・ ${n} cards`,
  epicAssign: 'Assign…',
  epicUnassigned: (name, n) => `${name}: ${n} unassigned`,
  stateConnected: 'Connected',
  stateUnconnected: 'Not connected',
  stateReplaced: 'Replaced',
  sessionOf: (tool, label) => `${tool} · ${label}`,
  idle: (n) => (n === 0 ? 'just now' : `${n} min ago`),
  noSession: 'No parent session',
  replacedNote: 'The earlier session, replaced by a switch. Its updates are rejected',
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
  agentNameTaken: 'This name is already in use',
  previousSession: 'Previous connection',
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
  assignSection: 'Agent',
  assignChange: 'Change agent',
  assignAssign: 'Assign an agent',
  assignGeneration: (n) => `Generation ${n}`,
  assignRunning: 'Running',
  assignDoneNote: 'The agent of a finished card cannot be changed',
  reassignTitle: 'Change agent',
  reassignScope: (id, title) => `#${id} ${title}`,
  reassignFrom: 'Current agent',
  reassignTo: 'New agent',
  reassignCurrent: 'Current',
  reassignLoad: (n) => `${n} in progress`,
  reassignCarry: 'What carries over',
  reassignCarryLane: (lane) => `The lane (stays in ${lane})`,
  reassignCarryPlan: 'The plan and acceptance conditions',
  reassignCarryResults: (n) => `${n} result${n === 1 ? '' : 's'}`,
  reassignCarryQuestion: '1 answer to a question not yet received',
  reassignStops: 'What stops',
  reassignStopsBody: (name) =>
    `${name}'s run and its writes to this card. Late updates are rejected.`,
  reassignWarnTitle: 'No parent session is connected to the new agent',
  reassignWarnBody:
    'The card stays in its lane and nobody works on it until a session connects. Change anyway?',
  reassignWarnAck: 'I know it waits until a session connects',
  reassignNeedAck: 'Confirm to change',
  reassignConfirm: (name) => `Hand over to ${name}`,
  reassignPick: 'Choose the new agent',
  bulkTitle: (epic) => `Assign the unassigned cards of ${epic}`,
  bulkLead:
    'Only cards with no agent are affected. Cards that already have one, and finished cards, stay as they are.',
  bulkAgent: 'Agent to assign',
  bulkWill: (n) => `Cards to assign (${n})`,
  bulkSkip: (n) => `Cards left as they are (${n})`,
  bulkSkipAssigned: (name) => `Already with ${name}`,
  bulkSkipDone: 'Finished',
  bulkAfter: (name, a, b) => `${name}'s open cards ${a} → ${b}`,
  bulkLaneKept: 'Lanes do not change',
  bulkLater: 'Cards added later are not included',
  bulkConfirm: (n, name) => `Assign ${n} card${n === 1 ? '' : 's'} to ${name}`,
  bulkNone: 'No unassigned cards',
  resultReassigned: (id, from, to) => `Handed #${id} over from ${from} to ${to}`,
  resultReassignedBody: (g0, g1) =>
    `Recorded in history · generation ${g0} → ${g1} · the new agent continues once connected`,
  resultBulk: (n, name) => `Assigned ${n} card${n === 1 ? '' : 's'} to ${name}`,
  resultBulkBody: (kept) => `${kept} cards were left as they were · assigned cards are marked`,
  freshMark: 'Just assigned',
  handedMark: 'Just handed over',
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
