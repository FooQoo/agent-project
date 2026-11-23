import { CheckCircle2Icon, GaugeIcon } from 'lucide-react';
import { useId, useMemo, useState } from 'react';
import { EpicLabel } from '../../features/epics/label/EpicLabel';
import type { DetailTab } from '../../features/tasks/detail/DetailTabs';
import { TaskCardView } from '../../features/tasks/card/TaskCard';
import { Button } from '@/shared/ui/button';
import { Drawer } from '@/shared/ui/drawer';
import { Header } from '../../shared/ui/app-header/Header';
import '../../pages/monitor/components/monitor-lanes.css';
import { AgentChip, AgentStrip, AssignSection } from './AgentParts';
import {
  type Fixture,
  type Lane,
  LANES,
  type MockCard,
  agentOf,
  buildFixture,
  countIn,
  summaryOf,
} from './data';
import { AgentsSurface, BulkSurface, ReassignSurface, SwitchSurface } from './Dialogs';
import { DetailPane } from './DetailPane';
import { useIsJa, useMockText } from './text';
import './assignment-mock.css';

export type DialogName = 'agents' | 'agentsAdd' | 'switch' | 'switchCode' | 'reassign' | 'bulk';

export interface Initial {
  openId?: number | null;
  tab?: DetailTab;
  lane?: Lane;
  dialog?: DialogName;
  /** The agent the switch dialog is about. */
  switchAgent?: string;
  /** The choice already made in the handover or the bulk dialog. */
  to?: string;
  epicId?: string;
  /** Shows the result of an operation that was just confirmed. */
  done?: 'reassign' | 'bulk';
}

export interface AssignmentScreenProps {
  variant?: 'desktop' | 'phone';
  initial?: Initial;
}

type Notice =
  | { kind: 'reassign'; id: number; from: string; to: string; g0: number; g1: number }
  | { kind: 'bulk'; ids: number[]; to: string; kept: number };

const handOver = (cards: MockCard[], id: number, to: string): MockCard[] =>
  cards.map((c) =>
    c.id === id
      ? { ...c, agentId: to, generation: c.generation + 1, running: false, fresh: true }
      : { ...c, fresh: false },
  );

const bulkAssign = (cards: MockCard[], epicId: string, to: string): MockCard[] =>
  cards.map((c) =>
    c.epicId === epicId && c.agentId === null
      ? { ...c, agentId: to, generation: c.generation + 1, fresh: true }
      : { ...c, fresh: false },
  );

/**
 * The agent-assignment mock: the board with an agent strip, the card detail with an assignment section,
 * the agents settings, the connection switch, the mid-way handover, and the epic bulk assign. Fixed data held
 * in memory; nothing is saved.
 */
export function AssignmentScreen({ variant = 'desktop', initial = {} }: AssignmentScreenProps) {
  const ja = useIsJa();
  const m = useMockText();
  const fixture = useMemo<Fixture>(() => buildFixture(ja), [ja]);
  const phone = variant === 'phone';
  const kind = phone ? 'sheet' : 'dialog';

  // biome-ignore lint/correctness/useExhaustiveDependencies: the initial state is read once per language
  const start = useMemo(() => {
    let cards = buildFixture(ja).cards;
    let notice: Notice | null = null;
    if (initial.done === 'reassign') {
      const card = cards.find((c) => c.id === 28);
      cards = handOver(cards, 28, initial.to ?? 'A-3');
      notice = {
        kind: 'reassign',
        id: 28,
        from: 'A-2',
        to: initial.to ?? 'A-3',
        g0: card?.generation ?? 2,
        g1: (card?.generation ?? 2) + 1,
      };
    }
    if (initial.done === 'bulk') {
      const epicId = initial.epicId ?? 'LOGIN-1';
      const ids = cards.filter((c) => c.epicId === epicId && c.agentId === null).map((c) => c.id);
      cards = bulkAssign(cards, epicId, initial.to ?? 'A-1');
      notice = {
        kind: 'bulk',
        ids,
        to: initial.to ?? 'A-1',
        kept: cards.filter((c) => c.epicId === epicId).length - ids.length,
      };
    }
    return { cards, notice };
  }, [ja]);

  const [cards, setCards] = useState<MockCard[]>(start.cards);
  const [notice, setNotice] = useState<Notice | null>(start.notice);
  const [openId, setOpenId] = useState<number | null>(initial.openId ?? null);
  const [tab, setTab] = useState<DetailTab>(initial.tab ?? 'description');
  const [lane, setLane] = useState<Lane>(initial.lane ?? 'doing');
  const [dialog, setDialog] = useState<DialogName | null>(initial.dialog ?? null);
  const [switchAgent, setSwitchAgent] = useState(initial.switchAgent ?? 'A-1');
  const [epicId, setEpicId] = useState(initial.epicId ?? 'LOGIN-1');
  const lanesId = useId();
  const open = cards.find((c) => c.id === openId) ?? null;
  const close = () => setDialog(null);

  const confirmReassign = (to: string) => {
    if (!open) return;
    setNotice({
      kind: 'reassign',
      id: open.id,
      from: open.agentId ?? '',
      to,
      g0: open.generation,
      g1: open.generation + 1,
    });
    setCards((all) => handOver(all, open.id, to));
    close();
  };
  const confirmBulk = (to: string) => {
    const ids = cards.filter((c) => c.epicId === epicId && c.agentId === null).map((c) => c.id);
    setNotice({
      kind: 'bulk',
      ids,
      to,
      kept: cards.filter((c) => c.epicId === epicId).length - ids.length,
    });
    setCards((all) => bulkAssign(all, epicId, to));
    close();
  };

  const laneCards = (name: Lane) => (
    <>
      {cards
        .filter((c) => c.lane === name)
        .map((c) => (
          <div key={c.id} className="arow" data-fresh={c.fresh || undefined}>
            <TaskCardView
              task={summaryOf(c, fixture.epics)}
              selected={openId === c.id}
              onSelect={() => {
                setOpenId(c.id);
                setTab('description');
              }}
              renderEpic={(epic) => (
                <EpicLabel epic={epic} className="card__epic" onClick={() => undefined} />
              )}
            >
              <span className="amarks">
                <AgentChip
                  agent={agentOf(fixture.agents, c.agentId)}
                  mark={c.fresh ? m.freshMark : undefined}
                />
              </span>
            </TaskCardView>
          </div>
        ))}
      {countIn(cards, name) === 0 && <p className="alane__empty">{m.laneEmpty}</p>}
    </>
  );

  const detail = (statusLabels: 'pc' | 'monitor') =>
    open && (
      <DetailPane
        fixture={fixture}
        card={open}
        tab={tab}
        onTab={setTab}
        onClose={() => setOpenId(null)}
        statusLabels={statusLabels}
        onToggleCondition={() => undefined}
        top={
          <AssignSection
            card={open}
            agent={agentOf(fixture.agents, open.agentId)}
            onChange={() => setDialog('reassign')}
          />
        }
      />
    );

  const noticeView = notice && (
    <div className="anotice" role="status">
      <CheckCircle2Icon aria-hidden="true" />
      <div className="anotice__text">
        {notice.kind === 'reassign' ? (
          <>
            <strong>
              {m.resultReassigned(
                notice.id,
                agentOf(fixture.agents, notice.from)?.name ?? '',
                agentOf(fixture.agents, notice.to)?.name ?? '',
              )}
            </strong>
            <span>{m.resultReassignedBody(notice.g0, notice.g1)}</span>
          </>
        ) : (
          <>
            <strong>
              {m.resultBulk(notice.ids.length, agentOf(fixture.agents, notice.to)?.name ?? '')} (
              {notice.ids.map((id) => `#${id}`).join(', ')})
            </strong>
            <span>{m.resultBulkBody(notice.kept)}</span>
          </>
        )}
      </div>
      <Button type="button" size="s" variant="outline" onClick={() => setNotice(null)}>
        {m.dismiss}
      </Button>
    </div>
  );

  const strip = (
    <AgentStrip
      agents={fixture.agents}
      cards={cards}
      epics={fixture.epics}
      onManage={() => setDialog('agents')}
      onEpic={(id) => {
        setEpicId(id);
        setDialog('bulk');
      }}
    />
  );

  // The slot beside the project name where INTERACTION-MODES shows the project's mode.
  const modeSlot = (
    <span className="amode" data-slot="mode">
      <GaugeIcon aria-hidden="true" />
      {m.modeLabel}: <strong>{m.modeValue}</strong>
    </span>
  );

  const switchTarget = agentOf(fixture.agents, switchAgent) ?? fixture.agents[0];
  const dialogs = (
    <>
      {(dialog === 'agents' || dialog === 'agentsAdd') && (
        <AgentsSurface
          kind={kind}
          agents={fixture.agents}
          cards={cards}
          adding={dialog === 'agentsAdd'}
          onSwitch={(id) => {
            setSwitchAgent(id);
            setDialog('switch');
          }}
          onClose={close}
        />
      )}
      {(dialog === 'switch' || dialog === 'switchCode') && switchTarget && (
        <SwitchSurface
          kind={kind}
          agent={switchTarget}
          cards={cards}
          showCode={dialog === 'switchCode'}
          onShowCode={() => setDialog('switchCode')}
          onClose={close}
        />
      )}
      {dialog === 'reassign' && open && (
        <ReassignSurface
          kind={kind}
          fixture={fixture}
          cards={cards}
          card={open}
          initialTo={initial.to}
          onClose={close}
          onConfirm={confirmReassign}
        />
      )}
      {dialog === 'bulk' && (
        <BulkSurface
          kind={kind}
          fixture={fixture}
          cards={cards}
          epicId={epicId}
          initialTo={initial.to}
          onClose={close}
          onConfirm={confirmBulk}
        />
      )}
    </>
  );

  return (
    <>
      {!phone ? (
        <div className="aapp">
          <Header canAdd onAdd={() => {}} versionSlot={modeSlot} />
          {strip}
          <main className="aboard">
            <div className="aboard__row">
              <div className="board__lanes aboard__lanes">
                {LANES.map((name) => (
                  <section
                    key={name}
                    className="lane alane"
                    data-column={name}
                    data-lane={name}
                    aria-label={m[name]}
                  >
                    <header className="lane-header">
                      <h2 className="lane-header__name">{m[name]}</h2>
                      <span className="lane-header__count">{countIn(cards, name)}</span>
                    </header>
                    <div className="lane__scroll">
                      <div className="lane__cards">{laneCards(name)}</div>
                    </div>
                  </section>
                ))}
              </div>
              {open && <div className="aboard__detail">{detail('pc')}</div>}
            </div>
          </main>
        </div>
      ) : (
        <main className="monitor-shell aphone">
          <Header settingsSize="l" versionSlot={modeSlot} />
          {strip}
          <div className="lane-tabs atabs" role="tablist" aria-label="lanes">
            {LANES.map((name) => (
              <button
                key={name}
                type="button"
                role="tab"
                aria-selected={lane === name}
                aria-controls={lanesId}
                className="lane-tabs__tab atabs__tab"
                data-lane={name}
                onClick={() => setLane(name)}
              >
                <span className="lane-tabs__name">{m[name]}</span>
                <strong className="lane-tabs__count">{countIn(cards, name)}</strong>
              </button>
            ))}
          </div>
          <div className="aphone__lane alane" id={lanesId} data-lane={lane}>
            {laneCards(lane)}
          </div>
          <Drawer
            open={open !== null && dialog === null}
            onOpenChange={(isOpen) => !isOpen && setOpenId(null)}
            identity={open ? `#${open.id}` : ''}
            title={open?.title ?? ''}
            className="adrawer"
          >
            {detail('monitor')}
          </Drawer>
        </main>
      )}
      {noticeView}
      {dialogs}
    </>
  );
}
