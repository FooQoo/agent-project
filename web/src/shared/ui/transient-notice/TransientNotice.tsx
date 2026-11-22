import { CircleAlert, CircleCheck, Info, X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import './TransientNotice.css';

const SHOW_MS = 6000;
const LEAVE_MS = 150;

export type TransientNoticeTone = 'success' | 'error' | 'info';

const ICONS = { success: CircleCheck, error: CircleAlert, info: Info } as const;

/**
 * A short message at the top of the screen (top right on a wide screen, top center on a phone) that
 * goes away by itself, or when the close button is pressed. It is for the outcome of an action that has
 * no dialog to report in, such as a move that was refused. `tone` picks the icon. It is drawn at the
 * body so that it stays above the board and any open sheet.
 */
export function TransientNotice({
  message,
  onDismiss,
  tone = 'error',
  durationMs = SHOW_MS,
}: {
  message: string | null;
  onDismiss: () => void;
  tone?: TransientNoticeTone;
  durationMs?: number;
}) {
  const { t } = useTranslation();
  const [leaving, setLeaving] = useState(false);
  const leaveTimer = useRef<number | undefined>(undefined);
  useEffect(() => {
    if (message === null) return;
    setLeaving(false);
    const timer = window.setTimeout(onDismiss, durationMs);
    return () => window.clearTimeout(timer);
  }, [message, onDismiss, durationMs]);
  useEffect(() => () => window.clearTimeout(leaveTimer.current), []);
  const close = useCallback(() => {
    const still = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    if (still) return onDismiss();
    setLeaving(true);
    leaveTimer.current = window.setTimeout(onDismiss, LEAVE_MS);
  }, [onDismiss]);
  if (message === null) return null;
  const Icon = ICONS[tone];
  return createPortal(
    <div
      className="transient-notice"
      data-tone={tone}
      data-leaving={leaving ? 'true' : undefined}
      role="status"
    >
      <Icon className="transient-notice__icon" size={18} aria-hidden="true" />
      <span className="transient-notice__text">{message}</span>
      <button
        type="button"
        className="transient-notice__close"
        aria-label={t('transientNotice.close')}
        onClick={close}
      >
        <X size={16} aria-hidden="true" />
      </button>
    </div>,
    document.body,
  );
}
