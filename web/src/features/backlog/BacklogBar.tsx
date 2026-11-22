import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import '../../shared/styles/backlog.css';
import { SELECTION_PARTS, type SelectionAction } from './selectionAction';

/**
 * The bar of the selection mode on the PC: how many cards are picked, "Cancel" and "Send to Todo (N)"
 * in two buttons of the same width, at the bottom of the Backlog lane. It is also the way to send by
 * keyboard or screen reader, where dragging is not. The phone uses BacklogSheet instead.
 */
export function BacklogBar({
  action = 'send',
  count,
  busy,
  onCancel,
  onSend,
}: {
  /** `send`: the Backlog's send to Todo; `archive`: the Done lane's archive. */
  action?: SelectionAction;
  count: number;
  busy?: boolean;
  onCancel: () => void;
  onSend: () => void;
}) {
  const { t } = useTranslation();
  const parts = SELECTION_PARTS[action];
  return (
    <section className="backlog-bar" data-variant="lane" aria-label={t(parts.barLabel)}>
      <span className="backlog-bar__count" role="status">
        {t(parts.selected, { count })}
      </span>
      <div className="backlog-bar__actions">
        <Button type="button" size="s" variant="outline" disabled={busy} onClick={onCancel}>
          {t('backlog.cancel')}
        </Button>
        <Button type="button" size="s" disabled={busy || count === 0} onClick={onSend}>
          <parts.Icon aria-hidden="true" />
          {t(parts.submit, { count })}
        </Button>
      </div>
    </section>
  );
}
