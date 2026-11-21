import { useState } from 'react';
import '../../shared/styles/modal.css';
import { Button } from '@/shared/ui/button';
import { DialogActions } from '@/shared/ui/dialog-actions';
import { RadioGroup } from '@/shared/ui/radio-group';
import { NONE, type MockTask, VERSIONS } from './data';
import { useMockText } from './text';
import { OptionRow, ScopeBadge, Surface } from './Parts';
import './release-plan-mock.css';

/** The project setting: which version is current. It never touches a task. */
export function ProjectSettings({
  kind,
  current,
  tasks,
  onSave,
  onClose,
}: {
  kind: 'dialog' | 'sheet';
  current: string | null;
  tasks: MockTask[];
  onSave: (value: string | null) => void;
  onClose: () => void;
}) {
  const m = useMockText();
  const [value, setValue] = useState(current ?? NONE);
  const count = (v: string) => tasks.filter((task) => task.planned === v).length;
  return (
    <Surface
      kind={kind}
      title={m.settingsTitle}
      onClose={onClose}
      footer={
        <DialogActions>
          <Button type="button" variant="outline" onClick={onClose}>
            {m.cancel}
          </Button>
          <Button
            type="button"
            onClick={() => {
              onSave(value === NONE ? null : value);
              onClose();
            }}
          >
            {m.settingsSave}
          </Button>
        </DialogActions>
      }
    >
      <section className="rp-section" aria-labelledby="rp-settings-release">
        <p className="rp-eyebrow" id="rp-settings-release">
          {m.settingsRelease}
        </p>
        <div className="rp-section__head">
          <h3 className="rp-section__title">{m.settingsCurrentTitle}</h3>
          <ScopeBadge scope="project">{m.currentBarLabel}</ScopeBadge>
        </div>
        <p className="rp-note">{m.settingsCurrentBody}</p>
        <RadioGroup value={value} onValueChange={setValue} className="rp-options">
          {VERSIONS.map((v) => (
            <OptionRow
              key={v}
              value={v}
              checked={value === v}
              label={v}
              hint={m.settingsCurrentCount(v, count(v))}
            />
          ))}
          <OptionRow value={NONE} checked={value === NONE} label={m.settingsCurrentNone} />
        </RadioGroup>
        <p className="rp-note rp-note--info">{m.settingsNote}</p>
      </section>
    </Surface>
  );
}
