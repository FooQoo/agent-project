import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import { Checkbox } from '@/shared/ui/checkbox';
import { Input } from '@/shared/ui/input';
import { saveMode, type ModeChange } from '../api';
import { IconLabel } from '@/shared/ui/icon-label';
import { MessagesSquareIcon, XIcon } from 'lucide-react';
import { DEFAULT_SCHEDULE, targetLabel, type ProjectMode, type Schedule } from '../model';
import { endBrainstormOn, setProjectMode, useProjectMode } from '../store';
import './mode.css';

/** The daily sleep settings and the controls for an active brainstorm. */
export function ModeSection({
  mode,
  disabled = false,
  failed = false,
  onEndBrainstorm,
  onFollowSchedule,
  onSaveSchedule,
}: {
  /** `null` while the mode is not known; settings are disabled until it loads. */
  mode: ProjectMode | null;
  disabled?: boolean;
  failed?: boolean;
  /** Ends the brainstorm (back to sparring); without it a brainstorm shows no End button. */
  onEndBrainstorm?: () => void;
  /** Forget the manual mode, so the daily window decides. */
  onFollowSchedule?: () => void;
  onSaveSchedule: (schedule: Schedule) => void;
}) {
  const { t } = useTranslation();
  const scheduleId = useId();
  const startId = useId();
  const endId = useId();
  const schedule = mode?.schedule ?? DEFAULT_SCHEDULE;
  const unavailable = disabled || mode === null;
  const canClearOverride = mode?.manual && (schedule.enabled || mode.mode === 'sleep');
  const saveSchedule = (next: Partial<Schedule>) => {
    const merged = { ...schedule, ...next };
    if (merged.start && merged.end && merged.start !== merged.end) onSaveSchedule(merged);
  };
  return (
    <section className="settings__section">
      <div className="mode-schedule" data-mode-schedule="">
        <label className="mode-schedule__toggle" htmlFor={scheduleId}>
          <Checkbox
            id={scheduleId}
            checked={schedule.enabled}
            disabled={unavailable}
            onCheckedChange={(checked) => saveSchedule({ enabled: checked === true })}
          />
          <span>{t('mode.scheduleHeading')}</span>
        </label>
        <div className="mode-schedule__times">
          <label htmlFor={startId}>
            <span>{t('mode.scheduleStart')}</span>
            <Input
              id={startId}
              type="time"
              value={schedule.start}
              disabled={unavailable}
              onChange={(event) => saveSchedule({ start: event.target.value })}
            />
          </label>
          <label htmlFor={endId}>
            <span>{t('mode.scheduleEnd')}</span>
            <Input
              id={endId}
              type="time"
              value={schedule.end}
              disabled={unavailable}
              onChange={(event) => saveSchedule({ end: event.target.value })}
            />
          </label>
        </div>
      </div>
      <p className="settings__note">{t('mode.scheduleNote')}</p>
      {canClearOverride && onFollowSchedule ? (
        <div className="mode-schedule__override">
          {schedule.enabled ? <p className="settings__note">{t('mode.manualHolds')}</p> : null}
          <Button
            type="button"
            variant="outline"
            size="s"
            disabled={disabled}
            onClick={onFollowSchedule}
          >
            {t(schedule.enabled ? 'mode.followSchedule' : 'mode.endSleep')}
          </Button>
        </div>
      ) : null}
      {mode?.mode === 'brainstorm' && mode.target ? (
        <div className="mode-brainstorm" data-mode-brainstorm="">
          <span className="mode-brainstorm__state">
            <MessagesSquareIcon aria-hidden="true" />
            {t('mode.brainstormOn', { target: targetLabel(mode.target) })}
          </span>
          {onEndBrainstorm ? (
            <Button
              type="button"
              variant="outline"
              size="m"
              disabled={disabled}
              onClick={onEndBrainstorm}
            >
              <IconLabel icon={XIcon}>{t('mode.endBrainstorm')}</IconLabel>
            </Button>
          ) : null}
        </div>
      ) : null}
      {failed ? (
        <p className="settings__mode-error" role="alert">
          {t('mode.saveError')}
        </p>
      ) : null}
    </section>
  );
}

/** Saves sleep settings through the local API and shows the saved values. */
export function ConnectedModeSection({ projectId }: { projectId: string }) {
  const mode = useProjectMode(projectId);
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);
  const save = (change: ModeChange) => {
    setSaving(true);
    setFailed(false);
    saveMode(projectId, change).then(
      (saved) => {
        setProjectMode(projectId, saved);
        setSaving(false);
      },
      () => {
        // A refused or failed save leaves the saved value as it was.
        setFailed(true);
        setSaving(false);
      },
    );
  };
  return (
    <ModeSection
      mode={mode}
      disabled={saving}
      failed={failed}
      onEndBrainstorm={() => {
        setSaving(true);
        setFailed(false);
        endBrainstormOn(projectId).then(
          () => setSaving(false),
          () => {
            setFailed(true);
            setSaving(false);
          },
        );
      }}
      onFollowSchedule={() => save({ mode: 'auto' })}
      onSaveSchedule={(schedule) => save({ schedule })}
    />
  );
}
