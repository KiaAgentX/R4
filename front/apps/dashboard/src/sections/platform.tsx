/**
 * Section: Platform — `GET/PUT /settings`, `GET /security`,
 * `POST /system/backup` (STEP1-AUDIT §2.5). Honesty rules: the settings PUT
 * is still 501 NOT_IMPLEMENTED server-side — the form is rendered, and the
 * REAL server answer (localized 501 text) is shown, never a fake success.
 * Backup is a two-step armed action; its result panel is the server's truth.
 */
import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t, type Locale } from '@agent-one/i18n/dashboard';
import type { AgentOneClient } from '@agent-one/api-client';
import type { SectionProps } from '../props.js';
import { formatBytes, formatDate } from '../format.js';
import { ArmButton, Card, EmptyState, ErrorState, PendingState, errorText } from '../ui.js';

interface BackupResult {
  file: string;
  sizeBytes: number;
  kept: number;
  createdAt: string;
}

export default function PlatformSection({ client, locale }: SectionProps) {
  const queryClient = useQueryClient();
  const [key, setKey] = useState('');
  const [value, setValue] = useState('');
  const [backup, setBackup] = useState<BackupResult | null>(null);

  const settings = useQuery({
    queryKey: ['dash-settings'],
    queryFn: async () => unwrap(await client.api.GET('/settings')),
    staleTime: 30_000,
  });
  const security = useQuery({
    queryKey: ['dash-security'],
    queryFn: async () => unwrap(await client.api.GET('/security')),
    staleTime: 30_000,
  });

  const putSetting = useMutation({
    mutationFn: async (input: { key: string; value: string }) =>
      unwrap(await client.api.PUT('/settings', { body: { key: input.key, value: input.value } })),
    onSuccess: () => {
      setKey('');
      setValue('');
      void queryClient.invalidateQueries({ queryKey: ['dash-settings'] });
    },
  });

  const runBackup = useMutation({
    mutationFn: async () => unwrap(await client.api.POST('/system/backup')),
    onSuccess: (data) => setBackup(data),
  });

  if (settings.isError) {
    return <ErrorState locale={locale} error={settings.error} onRetry={() => { void settings.refetch(); }} />;
  }
  if (security.isError) {
    return <ErrorState locale={locale} error={security.error} onRetry={() => { void security.refetch(); }} />;
  }
  if (settings.isPending || security.isPending) return <PendingState locale={locale} />;

  const settingItems = settings.data.data.items;
  const layers = security.data.data.layers;
  const audit = security.data.data.audit;
  const putError = putSetting.error;
  const backupError = runBackup.error;

  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'settingsTitle')} note={t(locale, 'settingsPutHonest')}>
        {settingItems.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table className="aur-table">
              <thead>
                <tr>
                  <th scope="col">{t(locale, 'settingKey')}</th>
                  <th scope="col">{t(locale, 'settingValue')}</th>
                </tr>
              </thead>
              <tbody>
                {settingItems.map((s) => (
                  <tr key={s.key}>
                    <td dir="ltr">
                      <code className="text-xs">{s.key}</code>
                    </td>
                    <td dir="auto">{s.value}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <div className="grid gap-3 sm:grid-cols-2">
          <div className="aur-field">
            <label className="aur-label" htmlFor="ao-setting-key">
              {t(locale, 'settingKey')}
            </label>
            <input
              id="ao-setting-key"
              className="aur-input"
              dir="ltr"
              value={key}
              onChange={(e) => setKey(e.target.value)}
            />
          </div>
          <div className="aur-field">
            <label className="aur-label" htmlFor="ao-setting-value">
              {t(locale, 'settingValue')}
            </label>
            <input
              id="ao-setting-value"
              className="aur-input"
              value={value}
              onChange={(e) => setValue(e.target.value)}
            />
          </div>
        </div>

        {putError !== null && (
          <div role="alert" className="aur-error-state">
            {errorText(locale, putError)}
          </div>
        )}

        <div>
          <button
            type="button"
            className="aur-btn aur-btn--primary"
            disabled={key.trim().length === 0 || putSetting.isPending}
            onClick={() => putSetting.mutate({ key: key.trim(), value })}
          >
            {t(locale, 'settingPut')}
          </button>
        </div>
      </Card>

      <Card title={t(locale, 'securityTitle')}>
        <h3 className="font-bold">{t(locale, 'securityLayers')}</h3>
        {layers.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table className="aur-table">
              <thead>
                <tr>
                  <th scope="col">{t(locale, 'colId')}</th>
                  <th scope="col">{t(locale, 'colName')}</th>
                  <th scope="col">{t(locale, 'colStatus')}</th>
                </tr>
              </thead>
              <tbody>
                {layers.map((l) => (
                  <tr key={l.id}>
                    <td dir="ltr">
                      <code className="text-xs">{l.id}</code>
                    </td>
                    <td>
                      {l.name}
                      {l.description !== undefined && (
                        <div className="text-aur-faint text-xs">{l.description}</div>
                      )}
                    </td>
                    <td>
                      {l.active ? (
                        <span className="aur-badge aur-badge--brand">{t(locale, 'layerActive')}</span>
                      ) : (
                        <span className="aur-badge">{t(locale, 'layerInactive')}</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <h3 className="font-bold">{t(locale, 'securityAudit')}</h3>
        {audit.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table className="aur-table">
              <thead>
                <tr>
                  <th scope="col">{t(locale, 'colActor')}</th>
                  <th scope="col">{t(locale, 'colAction')}</th>
                  <th scope="col">{t(locale, 'colTarget')}</th>
                  <th scope="col">{t(locale, 'colStatus')}</th>
                  <th scope="col">{t(locale, 'colDate')}</th>
                </tr>
              </thead>
              <tbody>
                {audit.map((a) => (
                  <tr key={a.id}>
                    <td dir="ltr">
                      <code className="text-xs">{a.actor}</code>
                    </td>
                    <td>{a.action}</td>
                    <td dir="ltr">
                      <code className="text-xs">{a.target ?? '—'}</code>
                    </td>
                    <td>
                      <span className="aur-badge">{a.status}</span>
                    </td>
                    <td>{formatDate(a.createdAt, locale)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>

      <Card title={t(locale, 'backupTitle')}>
        {backupError !== null && (
          <div role="alert" className="aur-error-state">
            {errorText(locale, backupError)}
          </div>
        )}
        <div className="flex flex-wrap gap-3">
          <ArmButton
            label={t(locale, 'backupRun')}
            armedLabel={t(locale, 'backupConfirm')}
            variant="primary"
            disabled={runBackup.isPending}
            onConfirm={() => runBackup.mutate()}
          />
        </div>
        {backup !== null && (
          <div role="status" className="aur-card__footer">
            {t(locale, 'backupFile')}:{' '}
            <code className="text-xs" dir="ltr">
              {backup.file}
            </code>{' '}
            · {t(locale, 'backupSize')}: {formatBytes(backup.sizeBytes)} ·{' '}
            {t(locale, 'backupKept')}: {backup.kept} · {t(locale, 'colDate')}:{' '}
            {formatDate(backup.createdAt, locale)}
          </div>
        )}
      </Card>
    </div>
  );
}
