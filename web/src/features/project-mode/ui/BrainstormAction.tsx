import { MessagesSquareIcon, XIcon } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import { IconLabel } from '@/shared/ui/icon-label';
import { type BrainstormTarget, type ProjectMode, sameTarget, targetLabel } from '../model';
import { endBrainstormOn, startBrainstormOn, useProjectMode } from '../store';
import './mode.css';

/**
 * The Brainstorm action of a card or an epic. While this target is brainstormed it says so and
 * offers to end; while another is, starting moves the brainstorm here (one target at a time).
 * Asleep, by the schedule or by hand, the product rule is that sleep wins, so it is disabled.
 */
export function BrainstormActionView({
  mode,
  target,
  busy = false,
  failed = false,
  onStart,
  onEnd,
}: {
  /** `null` while the mode is not known; the action is then disabled. */
  mode: ProjectMode | null;
  target: BrainstormTarget;
  busy?: boolean;
  failed?: boolean;
  onStart: () => void;
  onEnd: () => void;
}) {
  const { t } = useTranslation();
  const label = targetLabel(target);
  if (mode?.mode === 'brainstorm' && sameTarget(mode.target, target)) {
    return (
      <span className="brainstorm-action" data-brainstorm-action="active">
        <span className="brainstorm-action__state" role="status">
          <MessagesSquareIcon aria-hidden="true" />
          {t('mode.brainstorming')}
        </span>
        <Button type="button" variant="outline" size="s" disabled={busy} onClick={onEnd}>
          <IconLabel icon={XIcon}>{t('mode.endBrainstormShort')}</IconLabel>
        </Button>
      </span>
    );
  }
  const asleep = mode?.mode === 'sleep';
  const reason = asleep ? t('mode.brainstormUnavailableAsleep') : undefined;
  return (
    <span className="brainstorm-action" data-brainstorm-action="idle" title={reason}>
      <Button
        type="button"
        variant="outline"
        size="s"
        disabled={busy || mode === null || asleep}
        aria-label={t('mode.brainstormTarget', { target: label })}
        onClick={onStart}
      >
        <IconLabel icon={MessagesSquareIcon}>{t('mode.brainstorm')}</IconLabel>
      </Button>
      {asleep ? <span className="visually-hidden">{reason}</span> : null}
      {failed ? (
        <span className="brainstorm-action__error" role="alert">
          {t('mode.brainstormError')}
        </span>
      ) : null}
    </span>
  );
}

/** The action of one card or epic, wired to the local API. */
export function BrainstormAction({
  projectId,
  target,
}: {
  projectId: string;
  target: BrainstormTarget;
}) {
  const mode = useProjectMode(projectId);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const run = (action: () => Promise<void>) => {
    setBusy(true);
    setFailed(false);
    action().then(
      () => setBusy(false),
      () => {
        setFailed(true);
        setBusy(false);
      },
    );
  };
  return (
    <BrainstormActionView
      mode={mode}
      target={target}
      busy={busy}
      failed={failed}
      onStart={() => run(() => startBrainstormOn(projectId, target))}
      onEnd={() => run(() => endBrainstormOn(projectId))}
    />
  );
}
