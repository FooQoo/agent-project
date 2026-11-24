import { CheckCircle2Icon, CircleDashedIcon } from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { ResultPlanView } from '../../../shared/api/contracts';
import './result-plans.css';

/**
 * One planned result as a box on the Results tab: what is to be delivered and when, whether it is
 * delivered, and the result that delivered it inside (`children`). An undelivered box is empty and
 * says so. Results are registered by agents with `agpj result add --plan`.
 */
export function PlanBox({ plan, children }: { plan: ResultPlanView; children?: ReactNode }) {
  const { t } = useTranslation();
  const delivered = plan.deliveredResultVersion !== null;
  const state = delivered ? 'delivered' : 'pending';
  return (
    <section className="plan-box" data-plan={plan.id} data-state={state}>
      <div className="plan-box__meta">
        <span className="plan-box__id">{plan.id}</span>
        <span className="plan-box__kind" data-final={plan.final ? 'true' : undefined}>
          {plan.final ? t('resultPlans.final') : t('resultPlans.intermediate')}
        </span>
        <span className="plan-box__state" data-state={state}>
          {delivered ? (
            <CheckCircle2Icon aria-hidden="true" focusable="false" />
          ) : (
            <CircleDashedIcon aria-hidden="true" focusable="false" />
          )}
          {delivered ? t('resultPlans.delivered') : t('resultPlans.notDelivered')}
        </span>
      </div>
      <p className="plan-box__content">{plan.content}</p>
      <p className="plan-box__condition">
        <span className="plan-box__condition-label">{t('resultPlans.when')}</span>
        {plan.condition}
      </p>
      {delivered && <div className="plan-box__results">{children}</div>}
    </section>
  );
}
