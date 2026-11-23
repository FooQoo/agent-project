// モード切り替え（設定の行とヘッダーのスリープ表示）。
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
      '手動の設定です。いまの時間帯が終わるまで、時間帯の外なら変えるまで、そのままです。',
    followSchedule: 'スケジュールに従う',
    until: '{{time}} まで',
    saveError: 'モードを保存できませんでした。元のままです。',
    indicatorTitle: 'スリープ中: エージェントは質問せず、あなたの代わりに判断します。',
  },
};
