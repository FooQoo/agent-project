import { CheckCircle2Icon, CircleDashedIcon } from 'lucide-react';
import { useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import type { ResultPlanView } from '../../../shared/api/contracts';
import type { PlanDelivery } from './deliverPlan';
import { PlanRegisterForm } from './PlanRegisterForm';
import './result-plans.css';

/**
 * One planned result as a box on the Results tab: what is to be delivered and when, whether it is
 * delivered, and the result that delivered it inside (`children`). An undelivered box is empty and
 * says so; a person who may record results registers the real result from it.
 */
export function PlanBox({
  plan,
  canRegister,
  onRegister,
  children,
}: {
  plan: ResultPlanView;
  canRegister: boolean;
  onRegister?: (plan: ResultPlanView, delivery: PlanDelivery) => Promise<void>;
  children?: ReactNode;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
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
      {!delivered && canRegister && onRegister && !open && (
        <Button type="button" size="s" className="plan-box__action" onClick={() => setOpen(true)}>
          {t('resultPlans.registerResult')}
        </Button>
      )}
      {!delivered && canRegister && onRegister && open && (
        <PlanRegisterForm
          planId={plan.id}
          placeholder={plan.content}
          onCancel={() => setOpen(false)}
          onSubmit={async (delivery) => {
            await onRegister(plan, delivery);
            setOpen(false);
          }}
        />
      )}
    </section>
  );
}
