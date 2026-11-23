// The mode switch of the settings, the header's mode indicator, and the Brainstorm action.
export const modeEn = {
  mode: {
    heading: 'Mode',
    sparring: 'Sparring',
    sleep: 'Sleep',
    sparringNote:
      'The agent listens for requests and asks you on the board when it needs an answer.',
    sleepNote:
      'Questions to you are paused. The agent keeps working and decides in your place; each decision is recorded.',
    scheduleHeading: 'Sleep on a schedule',
    scheduleNote: 'Every day, in the local time of this computer. A window may cross midnight.',
    scheduleStart: 'From',
    scheduleEnd: 'Until',
    byScheduleUntil: 'Asleep by the schedule until {{time}}',
    manualHolds:
      'Set by hand: it holds until the current window ends, or until the next one starts when it was set outside a window.',
    followSchedule: 'Follow the schedule',
    until: 'until {{time}}',
    brainstorm: 'Brainstorm',
    brainstorming: 'Brainstorming',
    brainstormTarget: 'Brainstorm {{target}}',
    brainstormChip: 'Brainstorm: {{target}}',
    brainstormTitle: 'Brainstorm mode: the agent asks questions to deepen {{target}}.',
    brainstormOn: 'Brainstorming {{target}}',
    brainstormNote:
      'The agent asks you questions one at a time to deepen {{target}}. Choosing Sparring or Sleep ends it.',
    endBrainstorm: 'End brainstorm',
    endBrainstormShort: 'End',
    brainstormUnavailableAsleep: 'Brainstorming is unavailable in sleep mode.',
    brainstormError:
      'Brainstorming could not be started. Check that the card or epic still exists.',
    brainstormBadge: 'Brainstorm: {{target}}',
    saveError: 'The mode could not be saved. It stays as it was.',
    indicatorTitle: 'Sleep mode: the agent does not ask questions and decides in your place.',
  },
};
