export const versionsEn = {
  versions: {
    tag: {
      none: 'Unplanned',
      current: 'Current',
    },
    header: {
      button: 'Current version',
      buttonValue: 'Current version: {{version}}',
      unset: 'Not set',
      listLabel: 'Versions of this project',
      none: 'No current version',
      addLabel: 'New version',
      addPlaceholder: 'New version, such as v1.9.0',
      addConfirm: 'Add this version to the project',
      markCurrent: 'Current',
      empty: 'No versions yet. Type one above.',
      failed: 'Could not change the current version.',
    },
    detail: {
      title: 'Planned version',
      note: "Only this task changes. The project's current version ({{version}}) stays.",
      noteUnset: "Only this task changes. The project's current version stays unset.",
      current: 'Make it current ({{version}})',
      currentHint: "Saves the current version's name on this task.",
      currentUnset: 'Make it current',
      currentUnsetHint: 'Unavailable: no current version is set',
      none: 'Leave unplanned',
      failed: 'Could not save the planned version.',
      conflict:
        'Another change set the planned version to {{version}} first. Your choice was not saved. Choose again and save to replace it.',
      conflictNone:
        'Another change cleared the planned version first. Your choice was not saved. Choose again and save to set it.',
    },
    bulk: {
      title: "Planned version of this epic's tasks",
      note: 'Saving sets every task of the epic that is not done to this version. Change one task in its own editor.',
      current: 'Current ({{version}})',
      currentUnset: 'Current (unavailable: none set)',
      target: 'Planned version of the epic',
      noChange: 'No change',
    },
  },
} as const;
