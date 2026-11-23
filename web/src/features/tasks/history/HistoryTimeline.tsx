import { Suspense, useMemo } from 'react';
import { preloadable } from '../../../shared/lib/lazy/preloadable';
import type { AttachmentResolver, RelativeImageResolver } from '../../../shared/markdown/model';
import { formatShort } from '../model/format';
import { historyItems } from './model/history';
import type { HistoryEvent } from '../../../shared/api/contracts';

/** The Markdown renderer loads after the first screen; a reason shows as plain text until then. */
const blocksView = preloadable(() =>
  import('../../../shared/markdown/MarkdownBlocks').then((module) => module.MarkdownBlocks),
);

/** One recorded note, drawn as the task body is. */
function HistoryReason({
  reason,
  resolveAttachment,
  resolveImage,
}: {
  reason: string;
  resolveAttachment?: AttachmentResolver;
  resolveImage?: RelativeImageResolver;
}) {
  const blocks = useMemo(
    () => [{ id: 'reason', kind: 'markdown' as const, source: reason }],
    [reason],
  );
  return (
    <Suspense fallback={<p className="history__plain">{reason}</p>}>
      <blocksView.Component
        blocks={blocks}
        resolveAttachment={resolveAttachment}
        resolveImage={resolveImage}
        chapterHeadings
      />
    </Suspense>
  );
}

export function HistoryTimeline({
  events,
  resolveAttachment,
  resolveImage,
}: {
  events: HistoryEvent[];
  /** Turns an `attachment:<id>` image into its URL. */
  resolveAttachment?: AttachmentResolver;
  /** Turns an `attachments/<Q-n>/<name>` image copied from a question into its URL. */
  resolveImage?: RelativeImageResolver;
}) {
  const items = historyItems(events);
  return (
    <ol className="history">
      {items.map((item, index) => (
        <li key={events.length - index - 1} className="history__step">
          <span className="dot dot--history" data-tone="idle" aria-hidden="true" />
          <div className="history__text">
            <span className="history__at">{formatShort(item.at)}</span>
            <div className="history__reason">
              <HistoryReason
                reason={item.reason}
                resolveAttachment={resolveAttachment}
                resolveImage={resolveImage}
              />
            </div>
          </div>
        </li>
      ))}
    </ol>
  );
}
