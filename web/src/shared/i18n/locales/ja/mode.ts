// モード切り替え（設定の行、ヘッダーのモード表示、壁打ちの操作）。
export const modeJa = {
  mode: {
    heading: 'モード',
    sparring: '組手',
    sleep: 'スリープ',
    sparringNote: 'エージェントが依頼を待ち受け、答えが必要なときはボードで質問します。',
    sleepNote:
      '質問を止め、エージェントがあなたの代わりに判断して作業を続けます。判断は記録に残ります。',
    scheduleHeading: 'スケジュールでスリープ',
    scheduleNote: '毎日、このコンピューターの時刻で動きます。終了が開始より前なら日をまたぎます。',
    scheduleStart: '開始',
    scheduleEnd: '終了',
    byScheduleUntil: 'スケジュールにより {{time}} までスリープ',
    manualHolds:
      '手動の設定です。いまの時間帯が終わるまで、時間帯の外で設定したなら次の時間帯が始まるまで、そのままです。',
    followSchedule: 'スケジュールに従う',
    until: '{{time}} まで',
    brainstorm: '壁打ち',
    brainstorming: '壁打ち中',
    brainstormTarget: '壁打ち: {{target}}',
    brainstormChip: '壁打ち: {{target}}',
    brainstormTitle: '壁打ちモード: エージェントが {{target}} を深めるための質問をします。',
    brainstormOn: '壁打ち中: {{target}}',
    brainstormNote:
      'エージェントが {{target}} を深めるために、質問を1つずつします。組手かスリープを選ぶと終わります。',
    endBrainstorm: '壁打ちを終える',
    endBrainstormShort: '終える',
    brainstormUnavailableAsleep: 'スリープ中は壁打ちできません。',
    brainstormError:
      '壁打ちを始められませんでした。カードやエピックが残っているか確認してください。',
    brainstormBadge: '壁打ち: {{target}}',
    saveError: 'モードを保存できませんでした。元のままです。',
    indicatorTitle:
      'スリープ中: エージェントはあなたの代わりに判断し、質問はスリープが解けるまで保留します。',
    held_one: '保留中の質問 {{count}} 件',
    held_other: '保留中の質問 {{count}} 件',
    heldTitle_one: '保留中の質問が {{count}} 件あります。スリープが解けると届きます。',
    heldTitle_other: '保留中の質問が {{count}} 件あります。スリープが解けるとまとめて届きます。',
  },
};
