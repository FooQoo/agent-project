import { MoonIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ProjectMode } from '../model';
import { formatUntil } from '../time';
import { useProjectMode } from '../store';
import './mode.css';

/** The chip the header shows while a project is asleep. Sparring is the default and shows nothing. */
export function ModeIndicatorView({ mode }: { mode: ProjectMode | null }) {
  const { t, i18n } = useTranslation();
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
  return <ModeIndicatorView mode={useProjectMode(projectId)} />;
}
