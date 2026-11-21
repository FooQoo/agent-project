import { FlagIcon, FlagOffIcon, TagIcon, TriangleAlertIcon } from 'lucide-react';
import { useId, useMemo, useState } from 'react';
import { TaskCardView } from '../../features/tasks/card/TaskCard';
import { LaneHeader } from '../../pages/board/ui/LaneHeader';
import { LaneTabs } from '../../pages/monitor/components/LaneTabs';
import type { MonitorSection } from '../../pages/monitor/components/MonitorSectionHeader';
import { Header } from '../../shared/ui/app-header/Header';
import '../../shared/styles/modal.css';
import { Button } from '@/shared/ui/button';
import { buildTasks, DEFAULT_CURRENT, matches, type MockTask, type VersionFilter } from './data';
import { useIsJa, useMockText } from './text';
import { EpicBulk } from './EpicBulk';
import { FilterButton, FilterSheet, filterText } from './FilterControls';
import { VersionTag } from './Parts';
import { ProjectSettings } from './ProjectSettings';
import { TaskDetail } from './TaskDetail';
import './release-plan-mock.css';

/** What is open above the screen. */
export type Overlay =
  | { kind: 'none' }
  | { kind: 'filter' }
  | { kind: 'settings' }
  | { kind: 'detail'; taskId: number }
  | { kind: 'bulk'; epicId: string; target?: string | null; scope?: 'undecided' | 'all' };

export interface ReleasePlanScreenProps {
  variant?: 'desktop' | 'phone';
  /** The project's current version; `null` is "not set". */
  initialCurrent?: string | null;
  initialFilter?: VersionFilter;
  initial?: Overlay;
}

/** A card without an epic still draws its meta row (the version tag) through the epic slot. */
const NO_EPIC = { id: '', name: '', color: '#000000' };
const LANES = ['todo', 'doing', 'done'] as const;

/**
 * The release plan mock: the board (or the phone monitor) with a planned-version filter, the
 * project's current version, the task detail's version picker and the epic bulk change. Every
 * value is static; nothing is saved.
 */

export function ReleasePlanScreen({
  variant = 'desktop',
  initialCurrent = DEFAULT_CURRENT,
  initialFilter = { kind: 'all' },
  initial = { kind: 'none' },
}: ReleasePlanScreenProps) {
  const ja = useIsJa();
  const m = useMockText();
  const [tasks, setTasks] = useState<MockTask[]>(() => buildTasks(ja));
  // The task titles follow the language of the Story; the plans the user set stay.
  const shown = useMemo(() => {
    const fresh = buildTasks(ja);
    return tasks.map((task) => ({
      ...task,
      title: fresh.find((f) => f.id === task.id)?.title ?? '',
    }));
  }, [tasks, ja]);
  const [current, setCurrent] = useState<string | null>(initialCurrent);
  const [filter, setFilter] = useState<VersionFilter>(initialFilter);
  const [overlay, setOverlay] = useState<Overlay>(initial);
  const [lane, setLane] = useState<MonitorSection>('todo');
  const lanesId = useId();
  const kind = variant === 'phone' ? 'sheet' : 'dialog';
  const close = () => setOverlay({ kind: 'none' });

  const visible = shown.filter((task) => matches(task, filter, current));
  const laneTasks = (name: MonitorSection) => visible.filter((task) => task.taskStatus === name);
  const noCurrent = filter.kind === 'current' && current === null;
  const filtering = filter.kind !== 'all' && !noCurrent;
  const empty = filtering && visible.length === 0;

  const setPlanned = (ids: number[], planned: string | null) =>
    setTasks((prev) => prev.map((task) => (ids.includes(task.id) ? { ...task, planned } : task)));

  const settingsDialog = (onDone: () => void) => (
    <ProjectSettings
      kind={kind}
      current={current}
      tasks={shown}
      onSave={setCurrent}
      onClose={onDone}
    />
  );

  const bar = (
    <div className="rp-bar" data-variant={variant}>
      <div className="rp-bar__filter">
        <FilterButton
          m={m}
          filter={filter}
          current={current}
          variant={variant}
          onFilter={setFilter}
          onOpenSheet={() => setOverlay({ kind: 'filter' })}
        />
        <span className="rp-bar__count" role="status">
          {filtering ? m.filterShown(visible.length, shown.length) : m.filterShownAll(shown.length)}
        </span>
      </div>
      <div className="rp-bar__current" data-set={current !== null}>
        {current === null ? <FlagOffIcon aria-hidden="true" /> : <FlagIcon aria-hidden="true" />}
        <span className="rp-bar__label">{m.currentBarLabel}</span>
        <strong className="rp-bar__value">{current ?? m.currentBarUnset}</strong>
        <Button
          type="button"
          variant="outline"
          size="s"
          onClick={() => setOverlay({ kind: 'settings' })}
        >
          {current === null ? m.currentBarChange : m.currentBarEdit}
        </Button>
      </div>
    </div>
  );

  const notice = noCurrent && (
    <div className="rp-notice" role="alert">
      <TriangleAlertIcon aria-hidden="true" />
      <div className="rp-notice__text">
        <strong>{m.noCurrentTitle}</strong>
        <span>{m.noCurrentBody}</span>
      </div>
      <Button type="button" onClick={() => setOverlay({ kind: 'settings' })}>
        {m.noCurrentAction}
      </Button>
    </div>
  );

  const emptyPanel = empty && (
    <div className="rp-empty" role="status">
      <TagIcon aria-hidden="true" />
      <strong>{m.emptyTitle(filterText(m, filter, current))}</strong>
      <span>{m.emptyBody}</span>
      <Button type="button" variant="outline" onClick={() => setFilter({ kind: 'all' })}>
        {m.filterClear}
      </Button>
    </div>
  );

  const cards = (name: MonitorSection) => {
    const list = laneTasks(name);
    if (list.length === 0 && filtering) return <p className="rp-lane-empty">{m.emptyLane}</p>;
    return list.map((task) => (
      <TaskCardView
        key={task.id}
        task={{ ...task, epic: task.epic ?? NO_EPIC }}
        selected={false}
        onSelect={() => setOverlay({ kind: 'detail', taskId: task.id })}
        renderEpic={(epic) => (
          <span className="rp-card-row">
            {epic.id !== '' && (
              <span className="rp-epic" style={{ ['--rp-epic' as string]: epic.color }}>
                {epic.name}
              </span>
            )}
            <VersionTag planned={task.planned} current={current} />
          </span>
        )}
      />
    ));
  };

  const detailTask = overlay.kind === 'detail' ? shown.find((t) => t.id === overlay.taskId) : null;
  const settings = {
    render: ({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) =>
      open ? settingsDialog(() => onOpenChange(false)) : null,
  };

  return (
    <>
      {variant === 'desktop' ? (
        <div className="rp-app">
          <Header settings={settings} />
          <main className="board rp-board">
            {bar}
            {notice}
            {emptyPanel || (
              <div className="board__lanes">
                {LANES.map((name) => (
                  <section key={name} className="lane" data-column={name} aria-label={name}>
                    <LaneHeader column={name} count={laneTasks(name).length} />
                    <div className="lane__scroll">
                      <div className="lane__cards">{cards(name)}</div>
                    </div>
                  </section>
                ))}
              </div>
            )}
          </main>
        </div>
      ) : (
        <main className="monitor-shell">
          <Header settingsSize="l" settings={settings} />
          {bar}
          {notice}
          {emptyPanel || (
            <>
              <LaneTabs
                lane={lane}
                controlsId={lanesId}
                counts={{
                  backlog: 0,
                  todo: laneTasks('todo').length,
                  doing: laneTasks('doing').length,
                  done: laneTasks('done').length,
                }}
                onLaneChange={setLane}
              />
              <div className="rp-phone-lane" id={lanesId}>
                {cards(lane)}
              </div>
            </>
          )}
        </main>
      )}
      {overlay.kind === 'filter' && (
        <FilterSheet filter={filter} current={current} onPick={setFilter} onClose={close} />
      )}
      {overlay.kind === 'settings' && settingsDialog(close)}
      {detailTask && overlay.kind === 'detail' && (
        <TaskDetail
          kind={kind}
          task={detailTask}
          tasks={shown}
          current={current}
          onSave={(value) => setPlanned([detailTask.id], value)}
          onBulk={(epicId) => setOverlay({ kind: 'bulk', epicId })}
          onClose={close}
        />
      )}
      {overlay.kind === 'bulk' && (
        <EpicBulk
          kind={kind}
          epicId={overlay.epicId}
          tasks={shown}
          current={current}
          initialTarget={overlay.target === undefined ? 'v1.9.0' : overlay.target}
          initialScope={overlay.scope ?? 'undecided'}
          onApply={setPlanned}
          onClose={close}
        />
      )}
    </>
  );
}
