import { useState } from 'react';
import '../../shared/styles/modal.css';
import { Button } from '@/shared/ui/button';
import { DialogActions } from '@/shared/ui/dialog-actions';
import { RadioGroup } from '@/shared/ui/radio-group';
import { NONE, epicTasks, type MockTask, VERSIONS } from './data';
import { useMockText } from './text';
import { OptionRow, ScopeBadge, Surface, VersionTag } from './Parts';
import './release-plan-mock.css';

/** The task detail: the planned version of this one task, and the way into the epic's bulk change. */
export function TaskDetail({
  kind,
  task,
  tasks,
  current,
  onSave,
  onBulk,
  onClose,
}: {
  kind: 'dialog' | 'sheet';
  task: MockTask;
  tasks: MockTask[];
  current: string | null;
  onSave: (value: string | null) => void;
  onBulk: (epicId: string) => void;
  onClose: () => void;
}) {
  const m = useMockText();
  const [value, setValue] = useState(task.planned ?? NONE);
  const siblings = task.epic ? epicTasks(tasks, task.epic.id).filter((t) => t.id !== task.id) : [];
  const mixed = new Set(siblings.map((t) => t.planned)).size > 1;
  const others = VERSIONS.filter((v) => v !== current);
  return (
    <Surface
      kind={kind}
      title={task.title}
      identity={`#${task.id}`}
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
            {m.detailSave}
          </Button>
        </DialogActions>
      }
    >
      <section className="rp-section" aria-labelledby="rp-detail-plan">
        <div className="rp-section__head">
          <h3 className="rp-section__title" id="rp-detail-plan">
            {m.detailPlanTitle}
          </h3>
          <ScopeBadge scope="task">{`#${task.id}`}</ScopeBadge>
        </div>
        <p className="rp-note">
          {current === null ? m.detailPlanNoteUnset : m.detailPlanNote(current)}
        </p>
        <RadioGroup value={value} onValueChange={setValue} className="rp-options">
          <OptionRow
            value={current ?? '__current__'}
            checked={current !== null && value === current}
            disabled={current === null}
            label={current === null ? m.detailPickCurrentUnset : m.detailPickCurrent(current)}
            hint={current === null ? m.detailPickCurrentUnsetHint : m.detailPickCurrentHint}
          />
          {others.map((v) => (
            <OptionRow key={v} value={v} checked={value === v} label={v} />
          ))}
          <OptionRow value={NONE} checked={value === NONE} label={m.detailPickNone} />
        </RadioGroup>
      </section>
      {task.epic && siblings.length > 0 && (
        <section className="rp-section rp-epic-link" aria-label={m.detailEpicTitle(task.epic.name)}>
          <div className="rp-section__head">
            <h3 className="rp-section__title">{m.detailEpicTitle(task.epic.name)}</h3>
          </div>
          <ul className="rp-chips">
            {siblings.map((t) => (
              <li key={t.id}>
                <span className="rp-sibling">
                  <span className="rp-target__id">#{t.id}</span>
                  <VersionTag planned={t.planned} current={current} />
                </span>
              </li>
            ))}
          </ul>
          {mixed && <p className="rp-note">{m.detailEpicBody}</p>}
          <Button type="button" variant="outline" onClick={() => task.epic && onBulk(task.epic.id)}>
            {m.detailEpicAction}
          </Button>
        </section>
      )}
    </Surface>
  );
}
