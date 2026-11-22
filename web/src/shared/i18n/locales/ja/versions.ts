export const versionsJa = {
  versions: {
    tag: {
      none: '未定',
      current: 'カレント',
    },
    header: {
      button: 'カレント版',
      buttonValue: 'カレント版: {{version}}',
      unset: '未設定',
      listLabel: 'このプロジェクトの版',
      none: 'カレント版を決めない',
      addLabel: '新しい版',
      addPlaceholder: '新しい版（例: v1.9.0）',
      addConfirm: 'この版をプロジェクトに追加',
      markCurrent: 'カレント',
      empty: 'まだ版がありません。上に入力してください。',
      failed: 'カレント版を変更できませんでした。',
    },
    detail: {
      title: '予定版',
      note: '変わるのはこのタスクだけです。プロジェクトのカレント版（{{version}}）は変わりません。',
      noteUnset: '変わるのはこのタスクだけです。プロジェクトのカレント版は未設定のままです。',
      current: 'カレントにする（{{version}}）',
      currentHint: 'いまのカレント版の版名がこのタスクに保存されます。',
      currentUnset: 'カレントにする',
      currentUnsetHint: 'カレント版が未設定のため選べません',
      none: '未定にする',
      failed: '予定版を保存できませんでした。',
      conflict:
        '先に別の変更で予定版が {{version}} になりました。あなたの選択は保存していません。選び直して保存すると置き換えます。',
      conflictNone:
        '先に別の変更で予定版が未定になりました。あなたの選択は保存していません。選び直して保存すると設定します。',
    },
    bulk: {
      title: 'このエピックのタスクの予定版',
      note: '保存すると、完了していないエピックの全タスクがこの版になります。1つだけ変えるときは、そのタスクのエディターで。',
      current: 'カレント（{{version}}）',
      currentUnset: 'カレント（未設定のため選べません）',
      target: 'エピックの予定版',
      noChange: '変更しない',
    },
  },
} as const;
