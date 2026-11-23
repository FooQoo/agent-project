import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { RadioGroup as RadioGroupPrimitive } from 'radix-ui';
import { RadioGroup } from '@/shared/ui/radio-group';
import { Button } from '@/shared/ui/button';
import { Checkbox } from '@/shared/ui/checkbox';
import { Input } from '@/shared/ui/input';
import { saveMode, type ModeChange } from '../api';
import {
  DEFAULT_SCHEDULE,
  isModeKind,
  MODE_KINDS,
  SPARRING,
  type ModeKind,
  type ProjectMode,
  type Schedule,
} from '../model';
import { formatUntil } from '../time';
import { setProjectMode, useProjectMode } from '../store';
import './mode.css';

/** The switch and its note, with no data of its own: the connected one below feeds it. */
export function ModeSection({
  mode,
  disabled = false,
  failed = false,
  onChoose,
  onFollowSchedule,
  onSaveSchedule,
}: {
  /** `null` while the mode is not known; the switch then shows sparring and cannot be used. */
  mode: ProjectMode | null;
  disabled?: boolean;
  failed?: boolean;
  onChoose: (kind: ModeKind) => void;
  /** Forget the manual mode, so the daily window decides. */
  onFollowSchedule?: () => void;
  onSaveSchedule?: (schedule: Schedule) => void;
}) {
  const { t, i18n } = useTranslation();
  const headingId = useId();
  const scheduleId = useId();
  const startId = useId();
  const endId = useId();
  const schedule = mode?.schedule ?? DEFAULT_SCHEDULE;
  const unavailable = disabled || mode === null;
  const bySchedule = mode?.mode === 'sleep' && mode.modeSource === 'schedule';
  const saveSchedule = (next: Partial<Schedule>) => {
    const merged = { ...schedule, ...next };
    // A window needs two different times.
    if (merged.start !== merged.end) onSaveSchedule?.(merged);
  };
  const kind = mode?.mode ?? SPARRING.mode;
  return (
    <section className="settings__section settings__section--mode">
      <h3 className="settings__heading" id={headingId}>
        {t('mode.heading')}
      </h3>
      <RadioGroup
        className="segmented"
        data-mode-group=""
        aria-labelledby={headingId}
        value={kind}
        disabled={disabled || mode === null}
        onValueChange={(value) => {
          if (isModeKind(value)) onChoose(value);
        }}
      >
        {MODE_KINDS.map((value) => (
          <RadioGroupPrimitive.Item key={value} value={value} className="segmented__option">
            {t(`mode.${value}`)}
          </RadioGroupPrimitive.Item>
        ))}
      </RadioGroup>
      <p className="settings__note">
        {bySchedule && mode?.until
          ? t('mode.byScheduleUntil', { time: formatUntil(mode.until, i18n.language) })
          : t(kind === 'sleep' ? 'mode.sleepNote' : 'mode.sparringNote')}
      </p>
      {onSaveSchedule ? (
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
            <div>
              <label htmlFor={startId}>{t('mode.scheduleStart')}</label>
              <Input
                id={startId}
                type="time"
                value={schedule.start}
                disabled={unavailable}
                onChange={(event) => saveSchedule({ start: event.target.value })}
              />
            </div>
            <div>
              <label htmlFor={endId}>{t('mode.scheduleEnd')}</label>
              <Input
                id={endId}
                type="time"
                value={schedule.end}
                disabled={unavailable}
                onChange={(event) => saveSchedule({ end: event.target.value })}
              />
            </div>
          </div>
          <p className="settings__note">{t('mode.scheduleNote')}</p>
          {schedule.enabled && mode?.manual && onFollowSchedule ? (
            <>
              <p className="settings__note">{t('mode.manualHolds')}</p>
              <Button type="button" variant="outline" size="m" onClick={onFollowSchedule}>
                {t('mode.followSchedule')}
              </Button>
            </>
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

/** The mode switch wired to the local API: it saves on a choice and shows what the server says. */
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
      onChoose={(kind) => {
        if (kind !== mode?.mode || mode.modeSource !== 'manual') save({ mode: kind });
      }}
      onFollowSchedule={() => save({ mode: 'auto' })}
      onSaveSchedule={(schedule) => save({ schedule })}
    />
  );
}
