export const versionsJa = {
  versions: {
    tag: {
      none: '未定',
      current: 'カレント',
    },
    header: {
      button: 'カレントバージョン',
      buttonValue: 'カレントバージョン: {{version}}',
      unset: '未設定',
      listLabel: 'このプロジェクトのバージョン',
      none: 'カレントバージョンを決めない',
      addLabel: '新しいバージョン',
      addPlaceholder: '新しいバージョン（例: v1.9.0）',
      addConfirm: 'このバージョンをプロジェクトに追加',
      markCurrent: 'カレント',
      empty: 'まだバージョンがありません。上に入力してください。',
      failed: 'カレントバージョンを変更できませんでした。',
    },
    detail: {
      title: '予定バージョン',
      note: '変わるのはこのタスクだけです。プロジェクトのカレントバージョン（{{version}}）は変わりません。',
      noteUnset:
        '変わるのはこのタスクだけです。プロジェクトのカレントバージョンは未設定のままです。',
      current: 'カレントにする（{{version}}）',
      currentHint: 'いまのカレントバージョンの名前がこのタスクに保存されます。',
      currentUnset: 'カレントにする',
      currentUnsetHint: 'カレントバージョンが未設定のため選べません',
      none: '未定にする',
      failed: '予定バージョンを保存できませんでした。',
      conflict:
        '先に別の変更で予定バージョンが {{version}} になりました。あなたの選択は保存していません。選び直して保存すると置き換えます。',
      conflictNone:
        '先に別の変更で予定バージョンが未定になりました。あなたの選択は保存していません。選び直して保存すると設定します。',
    },
    bulk: {
      title: 'このエピックのタスクの予定バージョン',
      note: '保存すると、完了していないエピックの全タスクがこのバージョンになります。1つだけ変えるときは、そのタスクのエディターで。',
      current: 'カレント（{{version}}）',
      currentUnset: 'カレント（未設定のため選べません）',
      target: 'エピックの予定バージョン',
      noChange: '変更しない',
    },
  },
} as const;
