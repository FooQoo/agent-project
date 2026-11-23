import { ArrowRightIcon, CopyIcon } from 'lucide-react';
import { Button } from '@/shared/ui/button';
import { DialogActions } from '@/shared/ui/dialog-actions';
import { Avatar, SessionLine, StateBadge } from './AgentParts';
import type { MockAgent, MockCard } from './data';
import { Surface } from './Surface';
import { useMockText } from './text';
import type { Kind } from './surface-kind';
import './assignment-mock.css';

/**
 * Switching the parent session of an agent: the target and what happens to the current session come first,
 * and only then the one-time code appears (SPEC 6.1). The code is shown on the PC screen only.
 */
export function SwitchSurface({
  kind,
  agent,
  cards,
  showCode,
  onShowCode,
  onClose,
}: {
  kind: Kind;
  agent: MockAgent;
  cards: readonly MockCard[];
  showCode: boolean;
  onShowCode: () => void;
  onClose: () => void;
}) {
  const m = useMockText();
  const running = cards.filter((card) => card.agentId === agent.id && card.running).length;
  return (
    <Surface
      kind={kind}
      title={showCode ? m.switchCodeTitle : m.switchTitle}
      onClose={onClose}
      className="aswitch"
      footer={
        <DialogActions>
          <Button type="button" variant="outline" onClick={onClose}>
            {showCode ? m.close : m.cancel}
          </Button>
          {!showCode && (
            <Button type="button" onClick={onShowCode}>
              {m.switchShowCode}
            </Button>
          )}
        </DialogActions>
      }
    >
      <div className="adialog">
        <section>
          <h3 className="adialog__h">{m.switchTarget}</h3>
          <div className="atarget">
            <Avatar agent={agent} />
            <div className="atarget__text">
              <strong>{agent.name}</strong>
              <span>{agent.role}</span>
            </div>
          </div>
        </section>
        <div className="aswap" role="group" aria-label={`${m.switchNow} / ${m.switchAfter}`}>
          <div className="aswap__box">
            <span className="aswap__head">{m.switchNow}</span>
            {agent.session ? <SessionLine session={agent.session} /> : <span>{m.noSession}</span>}
            <StateBadge state={agent.state} />
          </div>
          <ArrowRightIcon aria-hidden="true" className="aswap__arrow" />
          <div className="aswap__box" data-after="true">
            <span className="aswap__head">{m.switchAfter}</span>
            <span>{m.switchAfterBody}</span>
          </div>
        </div>
        {showCode ? (
          <div className="acode">
            <p>{m.switchCodeBody}</p>
            <div className="acode__row">
              <code className="acode__cmd">agpj connect K7QM-4XD9</code>
              <Button type="button" size="s" variant="outline">
                <CopyIcon aria-hidden="true" />
              </Button>
            </div>
            <span className="acode__meta">{m.switchCodeExpires}</span>
            <p className="acode__wait">{m.switchCodeWait}</p>
          </div>
        ) : (
          <ul className="aeffects">
            {agent.session && <li>{m.switchEffect1}</li>}
            {running > 0 && <li>{m.switchEffect2(running)}</li>}
            <li>{m.switchEffect3}</li>
          </ul>
        )}
      </div>
    </Surface>
  );
}
