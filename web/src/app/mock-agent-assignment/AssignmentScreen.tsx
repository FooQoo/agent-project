import { GaugeIcon } from 'lucide-react';
import { useEffect, useId, useMemo, useState } from 'react';
import { EpicLabel } from '../../features/epics/label/EpicLabel';
import type { DetailTab } from '../../features/tasks/detail/DetailTabs';
import { TaskCardView } from '../../features/tasks/card/TaskCard';
import { Drawer } from '@/shared/ui/drawer';
import { Header, type SettingsSlotState } from '../../shared/ui/app-header/Header';
import '../../pages/monitor/components/monitor-lanes.css';
import { AssigneeButton, CardAgent, EpicAssignBar } from './AgentParts';
import {
  type Fixture,
  type Lane,
  LANES,
  type MockCard,
  agentOf,
  buildFixture,
  bulkTargets,
  countIn,
  summaryOf,
  openAiIn,
} from './data';
import { AgentsSurface } from './AgentsSurface';
import { BulkSurface } from './BulkSurface';
import { NoticeView } from './NoticeView';
import { ReassignSurface } from './ReassignSurface';
import { SwitchSurface } from './SwitchSurface';
import { DetailPane } from './DetailPane';
import {
  type DialogName,
  type Initial,
  type Notice,
  bulkAssign,
  handOver,
  initialState,
} from './screen-state';
import { useIsJa, useMockText } from './text';
import './assignment-mock.css';

export type { DialogName, Initial } from './screen-state';

/** Opens the agents surface when the product's settings button is pressed, then hands the press back. */
function SettingsBridge({
  open,
  onOpenChange,
  onOpenAgents,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onOpenAgents: () => void;
}) {
  useEffect(() => {
    if (!open) return;
    onOpenAgents();
    onOpenChange(false);
  }, [open, onOpenAgents, onOpenChange]);
  return null;
}

export interface AssignmentScreenProps {
  variant?: 'desktop' | 'phone';
  initial?: Initial;
}

/**
 * The agent-assignment mock: the board with the agent's avatar in each card, the card detail with the avatar in its status row,
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
  const start = useMemo(() => initialState(ja, initial), [ja]);

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
    const ids = bulkTargets(cards, epicId, to).map((c) => c.id);
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
              <CardAgent agent={agentOf(fixture.agents, c.agentId)} />
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
        extra={
          <>
            {!open.human && (
              <AssigneeButton
                agent={agentOf(fixture.agents, open.agentId)}
                disabled={open.lane === 'done'}
                onChange={() => setDialog('reassign')}
              />
            )}
            {tab === 'epic' && open.epicId && !open.human && (
              <EpicAssignBar
                count={openAiIn(cards, open.epicId).length}
                onAssign={() => {
                  setEpicId(open.epicId ?? epicId);
                  setDialog('bulk');
                }}
              />
            )}
          </>
        }
      />
    );

  const noticeView = notice && (
    <NoticeView notice={notice} agents={fixture.agents} onDismiss={() => setNotice(null)} />
  );

  // The slot beside the project name where INTERACTION-MODES shows the project's mode.
  const modeSlot = (
    <span className="amode" data-slot="mode">
      <GaugeIcon aria-hidden="true" />
      {m.modeLabel}: <strong>{m.modeValue}</strong>
    </span>
  );

  // Agents are managed in Settings only: the header's settings button opens the agents surface.
  const settings = {
    render: ({ open, onOpenChange }: SettingsSlotState) => (
      <SettingsBridge
        open={open}
        onOpenChange={onOpenChange}
        onOpenAgents={() => setDialog('agents')}
      />
    ),
  };

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
          <Header canAdd onAdd={() => {}} versionSlot={modeSlot} settings={settings} />
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
          <Header settingsSize="l" versionSlot={modeSlot} settings={settings} />
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
