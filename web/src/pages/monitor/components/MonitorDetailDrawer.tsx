import { useTranslation } from 'react-i18next';
import { RunTaskButton } from '../../../features/tasks/detail/RunTaskButton';
import { Drawer } from '../../../shared/ui/drawer';
import { MonitorDrawerPanels } from './MonitorDrawerPanels';
import type { Problem } from '../../../shared/api/contracts';
import type { MonitorTask, MonitorDetailTab, MonitorReadResolvers } from '../contracts';

export { shouldIgnoreTabSwipe } from '../hooks/useMonitorTabSwipe';

export function MonitorDetailDrawer({
  item,
  open,
  tab,
  onOpenChange,
  onTabChange,
  resolvers,
  problems = [],
  focusResultVersion = null,
}: {
  item: MonitorTask | null;
  open: boolean;
  tab: MonitorDetailTab;
  onOpenChange(open: boolean): void;
  onTabChange(tab: MonitorDetailTab): void;
  resolvers: MonitorReadResolvers;
  problems?: readonly (Problem & { projectId: string })[];
  focusResultVersion?: number | null;
}) {
  const { t } = useTranslation();
  if (!item) {
    // Mount the root closed so the first interaction also runs the enter transition.
    return (
      <Drawer open={false} onOpenChange={onOpenChange} identity="" title="">
        {null}
      </Drawer>
    );
  }

  const task = item.task;
  return (
    <Drawer
      open={open}
      onOpenChange={onOpenChange}
      identity={t('monitor.drawerIdentity', { id: task.id })}
      title={task.title}
      label={
        <div className="-mr-11 flex items-center justify-between gap-2 self-stretch">
          {task.epic ? (
            <span className="mobile-drawer__epic-label" title={task.epic.name}>
              {task.epic.id}
            </span>
          ) : (
            <span />
          )}
          <RunTaskButton projectId={item.projectId} task={task} compact />
        </div>
      }
    >
      <MonitorDrawerPanels
        item={item}
        tab={tab}
        onTabChange={onTabChange}
        resolvers={resolvers}
        problems={problems}
        focusResultVersion={focusResultVersion}
      />
    </Drawer>
  );
}
