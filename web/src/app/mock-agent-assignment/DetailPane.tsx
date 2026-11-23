import { type ReactNode, useMemo } from 'react';
import { TaskDetailView } from '../../features/tasks/detail/TaskDetailView';
import {
  createDetailFixtureAdapter,
  type DetailFixture,
} from '../../features/tasks/detail/detailFixture';
import type { DetailTab } from '../../features/tasks/detail/DetailTabs';
import { EpicLabel } from '../../features/epics/label/EpicLabel';
import { DataProvider, useEpicDetail, useTaskDetail } from '../../shared/data/react';
import { type Fixture, type MockCard, PROJECT_ID, summaryOf } from './data';
import './assignment-mock.css';

function fixtureOf(fixture: Fixture, card: MockCard): DetailFixture {
  return {
    tasks: {
      [card.id]: {
        blocks: [card.body],
        acceptance: card.conditions,
        history: card.history.map((reason, index) => ({
          at: `2026-11-24T0${index}:00:00.000Z`,
          reason,
        })),
      },
    },
    epics: Object.fromEntries(
      fixture.epics.map((epic) => [epic.id, { blocks: [fixture.epicBody[epic.id] ?? ''] }]),
    ),
  };
}

function Body({
  fixture,
  card,
  tab,
  onTab,
  onClose,
  onToggleCondition,
  statusLabels,
  top,
}: DetailPaneProps) {
  const summary = summaryOf(card, fixture.epics);
  const detail = useTaskDetail(PROJECT_ID, card.id);
  const epicDetail = useEpicDetail(PROJECT_ID, card.epicId);
  return (
    <div className="adetail">
      {top}
      <TaskDetailView
        projectId={PROJECT_ID}
        task={summary}
        document={detail.document}
        history={detail.history}
        epicDocument={epicDetail.document}
        tab={tab}
        onTabChange={onTab}
        onClose={onClose}
        epics={fixture.epics}
        statusLabels={statusLabels}
        readOnlyDraft={false}
        onToggleCondition={async (id, done) => onToggleCondition(id, done)}
        renderEpicLabel={(epic) => (
          <EpicLabel epic={epic} className="card__epic" onClick={() => undefined} />
        )}
      />
    </div>
  );
}

export interface DetailPaneProps {
  fixture: Fixture;
  card: MockCard;
  tab: DetailTab;
  onTab: (tab: DetailTab) => void;
  onClose: () => void;
  onToggleCondition: (conditionId: string, done: boolean) => void;
  /** The assignment section the mock adds above the product's detail. */
  top?: ReactNode;
  statusLabels?: 'pc' | 'monitor';
}

/**
 * The product's card detail (TaskDetailView with its tabs), reading the card from memory. The data
 * layer is re-created when the card's text changes, so a ticked condition shows at once.
 */
export function DetailPane(props: DetailPaneProps) {
  const { fixture, card } = props;
  const version = JSON.stringify([card.id, card.title, card.body, card.conditions, card.history]);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the version stands for the card's text
  const adapter = useMemo(() => createDetailFixtureAdapter(fixtureOf(fixture, card)), [version]);
  return (
    <DataProvider key={version} adapter={adapter}>
      <Body {...props} />
    </DataProvider>
  );
}
