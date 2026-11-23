import { MessagesSquareIcon, MoonIcon, XIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import { targetLabel, type ProjectMode } from '../model';
import { formatUntil } from '../time';
import { endBrainstormOn, useProjectMode } from '../store';
import './mode.css';

/**
 * The chip the header shows while a project is asleep or brainstorming. Sparring is the default
 * and shows nothing. A brainstorm names its target and can be ended from the chip.
 */
export function ModeIndicatorView({
  mode,
  onEndBrainstorm,
  endDisabled = false,
}: {
  mode: ProjectMode | null;
  onEndBrainstorm?: () => void;
  endDisabled?: boolean;
}) {
  const { t, i18n } = useTranslation();
  if (mode?.mode === 'brainstorm' && mode.target) {
    const target = targetLabel(mode.target);
    return (
      <span className="mode-indicator-group">
        <span
          className="mode-indicator mode-indicator--brainstorm"
          role="status"
          title={t('mode.brainstormTitle', { target })}
        >
          <MessagesSquareIcon aria-hidden="true" />
          <span className="mode-indicator__label">{t('mode.brainstormChip', { target })}</span>
        </span>
        {onEndBrainstorm ? (
          <Button
            type="button"
            variant="outline"
            size="s"
            iconOnly
            className="mode-indicator__end"
            aria-label={t('mode.endBrainstorm')}
            title={t('mode.endBrainstorm')}
            disabled={endDisabled}
            onClick={onEndBrainstorm}
          >
            <XIcon aria-hidden="true" />
          </Button>
        ) : null}
      </span>
    );
  }
  if (mode?.mode !== 'sleep') return null;
  return (
    <span className="mode-indicator" role="status" title={t('mode.indicatorTitle')}>
      <MoonIcon aria-hidden="true" />
      <span>{t('mode.sleep')}</span>
      {mode.modeSource === 'schedule' && mode.until ? (
        <span className="mode-indicator__detail">
          {t('mode.until', { time: formatUntil(mode.until, i18n.language) })}
        </span>
      ) : null}
      <span className="visually-hidden">{t('mode.indicatorTitle')}</span>
    </span>
  );
}

/** The indicator of one project, kept current by the board's `mode` events. */
export function ModeIndicator({ projectId }: { projectId: string | null }) {
  const mode = useProjectMode(projectId);
  return (
    <ModeIndicatorView
      mode={mode}
      onEndBrainstorm={
        projectId ? () => void endBrainstormOn(projectId).catch(() => undefined) : undefined
      }
    />
  );
}
