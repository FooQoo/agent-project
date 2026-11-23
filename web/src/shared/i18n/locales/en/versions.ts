export const versionsEn = {
  versions: {
    tag: {
      none: 'Unplanned',
      release: 'Release version',
    },
    header: {
      buttonValue: 'Release version: {{version}}',
      unset: 'Not set',
      listLabel: 'Versions of this project',
      none: 'No release version',
      addLabel: 'New version',
      addPlaceholder: 'New version, such as v1.9.0',
      addConfirm: 'Add this version to the project',
      markRelease: 'Release version',
      empty: 'No versions yet. Type one above.',
      failed: 'Could not change the release version.',
    },
    detail: {
      title: 'Planned version',
      note: "Only this task changes. The project's release version ({{version}}) stays.",
      noteUnset: "Only this task changes. The project's release version stays unset.",
      release: 'Make it the release version ({{version}})',
      releaseHint: "Saves the release version's name on this task.",
      releaseUnset: 'Make it the release version',
      releaseUnsetHint: 'Unavailable: no release version is set',
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
      release: 'Release version ({{version}})',
      releaseUnset: 'Release version (unavailable: none set)',
      target: 'Planned version of the epic',
      noChange: 'No change',
    },
  },
} as const;
