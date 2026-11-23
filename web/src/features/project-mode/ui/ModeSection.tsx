import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { RadioGroup as RadioGroupPrimitive } from 'radix-ui';
import { RadioGroup } from '@/shared/ui/radio-group';
import { saveMode } from '../api';
import { isModeKind, MODE_KINDS, SPARRING, type ModeKind, type ProjectMode } from '../model';
import { setProjectMode, useProjectMode } from '../store';
import './mode.css';

/** The switch and its note, with no data of its own: the connected one below feeds it. */
export function ModeSection({
  mode,
  disabled = false,
  failed = false,
  onChoose,
}: {
  /** `null` while the mode is not known; the switch then shows sparring and cannot be used. */
  mode: ProjectMode | null;
  disabled?: boolean;
  failed?: boolean;
  onChoose: (kind: ModeKind) => void;
}) {
  const { t } = useTranslation();
  const headingId = useId();
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
        {t(kind === 'sleep' ? 'mode.sleepNote' : 'mode.sparringNote')}
      </p>
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
  const choose = (kind: ModeKind) => {
    if (kind === mode?.mode) return;
    setSaving(true);
    setFailed(false);
    saveMode(projectId, kind).then(
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
  return <ModeSection mode={mode} disabled={saving} failed={failed} onChoose={choose} />;
}
