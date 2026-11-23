import { CheckCircle2Icon } from 'lucide-react';
import { Button } from '@/shared/ui/button';
import { type MockAgent, agentOf } from './data';
import type { Notice } from './screen-state';
import { useMockText } from './text';
import './assignment-mock.css';

/** The result of the operation that was just confirmed, with a button to dismiss it. */
export function NoticeView({
  notice,
  agents,
  onDismiss,
}: {
  notice: Notice;
  agents: readonly MockAgent[];
  onDismiss: () => void;
}) {
  const m = useMockText();
  return (
    <div className="anotice" role="status">
      <CheckCircle2Icon aria-hidden="true" />
      <div className="anotice__text">
        {notice.kind === 'reassign' ? (
          <>
            <strong>
              {m.resultReassigned(
                notice.id,
                agentOf(agents, notice.from)?.name ?? '',
                agentOf(agents, notice.to)?.name ?? '',
              )}
            </strong>
            <span>{m.resultReassignedBody(notice.g0, notice.g1)}</span>
          </>
        ) : (
          <>
            <strong>
              {m.resultBulk(notice.ids.length, agentOf(agents, notice.to)?.name ?? '')} (
              {notice.ids.map((id) => `#${id}`).join(', ')})
            </strong>
            <span>{m.resultBulkBody(notice.kept)}</span>
          </>
        )}
      </div>
      <Button type="button" size="s" variant="outline" onClick={onDismiss}>
        {m.dismiss}
      </Button>
    </div>
  );
}
