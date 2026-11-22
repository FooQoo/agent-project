import { useId } from 'react';
import { useTranslation } from 'react-i18next';
import { Label } from '@/shared/ui/label';
import { RadioGroup, RadioGroupItem } from '@/shared/ui/radio-group';
import type { Provider } from '../api';
import './connection.css';

/** The choice between ngrok and Tailscale; the settings of the other way are not shown. */
export function ProviderChoice({
  value,
  busy,
  onChange,
}: {
  value: Provider;
  busy: boolean;
  onChange: (provider: Provider) => void;
}) {
  const { t } = useTranslation();
  const ids = { ngrok: useId(), tailscale: useId() };
  const labelId = useId();
  return (
    <div className="connection-dialog__provider">
      <p className="connection-dialog__provider-label" id={labelId}>
        {t('monitorConnection.provider.label')}
      </p>
      <RadioGroup
        aria-labelledby={labelId}
        value={value}
        disabled={busy}
        onValueChange={(next) => onChange(next === 'tailscale' ? 'tailscale' : 'ngrok')}
      >
        {(['ngrok', 'tailscale'] as const).map((provider) => (
          <div key={provider} className="connection-dialog__provider-option">
            <RadioGroupItem value={provider} id={ids[provider]} />
            <Label htmlFor={ids[provider]}>
              {t(`monitorConnection.provider.${provider}`)}
              <span className="connection-dialog__provider-hint">
                {t(`monitorConnection.provider.${provider}Hint`)}
              </span>
            </Label>
          </div>
        ))}
      </RadioGroup>
    </div>
  );
}
