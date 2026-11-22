import { ArchiveIcon, SendIcon } from 'lucide-react';

/** What the selection bar and the phone's drawer do with the picked cards. */
export type SelectionAction = 'send' | 'archive';

/** The wording and the icon of each action. The bar and the drawer share them. */
export const SELECTION_PARTS = {
  send: {
    barLabel: 'backlog.barLabel',
    selected: 'backlog.selected',
    submit: 'backlog.send',
    Icon: SendIcon,
  },
  archive: {
    barLabel: 'archive.selection.barLabel',
    selected: 'archive.selection.selected',
    submit: 'archive.selection.submit',
    Icon: ArchiveIcon,
  },
} as const;
