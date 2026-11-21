import { useState } from 'react';
import '../../shared/styles/modal.css';
import { Button } from '@/shared/ui/button';
import { Checkbox } from '@/shared/ui/checkbox';
import { DialogActions } from '@/shared/ui/dialog-actions';
import { RadioGroup } from '@/shared/ui/radio-group';
import { NONE, epicTasks, type MockTask, VERSIONS } from './data';
import { useMockText } from './text';
import { OptionRow, PillRow, Surface } from './Parts';
import './release-plan-mock.css';

/** The epic bulk change: the targets, their count and each change, shown before anything is saved. */
export function EpicBulk({
  kind,
  epicId,
  tasks,
  current,
  initialTarget,
  initialScope,
  onApply,
  onClose,
}: {
  kind: 'dialog' | 'sheet';
  epicId: string;
  tasks: MockTask[];
  current: string | null;
  initialTarget: string | null;
  initialScope: 'undecided' | 'all';
  onApply: (ids: number[], target: string | null) => void;
  onClose: () => void;
}) {
  const m = useMockText();
  const members = epicTasks(tasks, epicId);
  const epic = members[0]?.epic ?? null;
  const [target, setTarget] = useState(initialTarget ?? NONE);
  const [scope, setScope] = useState<'undecided' | 'all'>(initialScope);
  const inScope = (task: MockTask) => scope === 'all' || task.planned === null;
  const [off, setOff] = useState<Set<number>>(new Set());
  const to = target === NONE ? null : target;
  const toText = to ?? m.plannedNone;
  const targeted = members.filter((task) => inScope(task) && !off.has(task.id));
  const changed = targeted.filter((task) => task.planned !== to);
  const undecided = members.filter((task) => task.planned === null).length;
  const targets = [...VERSIONS, NONE];
  return (
    <Surface
      kind={kind}
      title={m.bulkTitle}
      identity={epic?.name}
      onClose={onClose}
      className="rp-bulk"
      footer={
        <>
          <p
            className="rp-summary"
            data-empty={changed.length === 0}
            role="status"
            aria-live="polite"
          >
            {changed.length === 0
              ? m.bulkSummaryNone
              : m.bulkSummary(changed.length, members.length, toText)}
          </p>
          <DialogActions>
            <Button type="button" variant="outline" onClick={onClose}>
              {m.bulkCancel}
            </Button>
            <Button
              type="button"
              disabled={changed.length === 0}
              onClick={() => {
                onApply(
                  changed.map((task) => task.id),
                  to,
                );
                onClose();
              }}
            >
              {m.bulkApply}
            </Button>
          </DialogActions>
        </>
      }
    >
      <section className="rp-section">
        <h3 className="rp-section__title">{m.bulkTarget}</h3>
        <RadioGroup
          value={target}
          onValueChange={setTarget}
          className="rp-pills"
          aria-label={m.bulkTarget}
        >
          {targets.map((v) => (
            <PillRow
              key={v}
              value={v}
              checked={target === v}
              label={v === NONE ? m.plannedNone : v}
              mark={v === current ? m.plannedCurrentMark : undefined}
            />
          ))}
        </RadioGroup>
      </section>
      <section className="rp-section">
        <h3 className="rp-section__title">{m.bulkScope}</h3>
        <RadioGroup
          value={scope}
          onValueChange={(v) => {
            setScope(v === 'all' ? 'all' : 'undecided');
            setOff(new Set());
          }}
          className="rp-options"
          aria-label={m.bulkScope}
        >
          <OptionRow
            value="undecided"
            checked={scope === 'undecided'}
            label={m.bulkScopeUndecided(undecided)}
          />
          <OptionRow value="all" checked={scope === 'all'} label={m.bulkScopeAll(members.length)} />
        </RadioGroup>
      </section>
      <section className="rp-section">
        <h3 className="rp-section__title">{m.bulkListTitle}</h3>
        <p className="rp-note">{m.bulkListHint}</p>
        <ul className="rp-targets">
          {members.map((task) => {
            const eligible = inScope(task);
            const checked = eligible && !off.has(task.id);
            const same = task.planned === to;
            return (
              <li
                key={task.id}
                className="rp-target"
                data-state={!checked ? 'off' : same ? 'same' : 'change'}
              >
                <label htmlFor={`rp-t-${task.id}`} className="rp-target__label">
                  <Checkbox
                    id={`rp-t-${task.id}`}
                    checked={checked}
                    disabled={!eligible}
                    onCheckedChange={(next) =>
                      setOff((prev) => {
                        const copy = new Set(prev);
                        if (next === true) copy.delete(task.id);
                        else copy.add(task.id);
                        return copy;
                      })
                    }
                  />
                  <span className="rp-target__title">
                    <span className="rp-target__id">#{task.id}</span> {task.title}
                  </span>
                </label>
                <span className="rp-target__change">
                  <span className="rp-from">{task.planned ?? m.plannedNone}</span>
                  {checked && !same && (
                    <>
                      <span aria-hidden="true">→</span>
                      <strong>{toText}</strong>
                    </>
                  )}
                  {checked && same && <span className="rp-same">{m.bulkUnchanged}</span>}
                </span>
              </li>
            );
          })}
        </ul>
      </section>
    </Surface>
  );
}
