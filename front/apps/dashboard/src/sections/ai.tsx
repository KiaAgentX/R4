/**
 * Section: AI console — the admin usage meter over `GET /ai/usage` plus the
 * public registries `GET /ai/agents` (6 Zenovix agents) and `GET /ai/skills`
 * (16 KIA-OS skills). The daily cap gate lives on the server; here it is
 * rendered honestly with a progress meter.
 */
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t, type Locale } from '@agent-one/i18n/dashboard';
import type { AgentOneClient } from '@agent-one/api-client';
import type { SectionProps } from '../props.js';
import { skillKindLabel } from '../format.js';
import { Card, EmptyState, ErrorState, Meter, PendingState, Tile } from '../ui.js';

export default function AiConsoleSection({ client, locale }: SectionProps) {
  const [userFilter, setUserFilter] = useState('');

  const usage = useQuery({
    queryKey: ['dash-ai-usage', userFilter],
    queryFn: async () => {
      const trimmed = userFilter.trim();
      const query = trimmed.length > 0 ? { userId: trimmed } : {};
      return unwrap(await client.api.GET('/ai/usage', { params: { query } }));
    },
    staleTime: 15_000,
  });
  const agents = useQuery({
    queryKey: ['dash-ai-agents'],
    queryFn: async () => unwrap(await client.reads.agents()),
    staleTime: 5 * 60_000,
  });
  const skills = useQuery({
    queryKey: ['dash-ai-skills'],
    queryFn: async () => unwrap(await client.reads.skills()),
    staleTime: 5 * 60_000,
  });

  if (usage.isError) {
    return <ErrorState locale={locale} error={usage.error} onRetry={() => { void usage.refetch(); }} />;
  }
  if (agents.isError) {
    return <ErrorState locale={locale} error={agents.error} onRetry={() => { void agents.refetch(); }} />;
  }
  if (skills.isError) {
    return <ErrorState locale={locale} error={skills.error} onRetry={() => { void skills.refetch(); }} />;
  }
  if (usage.isPending || agents.isPending || skills.isPending) {
    return <PendingState locale={locale} />;
  }

  const u = usage.data;

  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'aiUsageTitle')} note={t(locale, 'aiIdentityNote')}>
        <div className="aur-field">
          <label className="aur-label" htmlFor="ao-ai-user">
            {t(locale, 'aiUsageFilter')}
          </label>
          <input
            id="ao-ai-user"
            className="aur-input"
            dir="ltr"
            type="text"
            value={userFilter}
            onChange={(e) => setUserFilter(e.target.value)}
          />
        </div>
        <div className="aur-tiles">
          <Tile label={t(locale, 'colUser')} value={<span dir="ltr"><code className="text-sm">{u.userId}</code></span>} />
          <Tile label={t(locale, 'aiUsedToday')} value={u.usedToday} />
          <Tile label={t(locale, 'aiCap')} value={u.cap} />
          <Tile label={t(locale, 'aiRemaining')} value={u.remainingToday} />
        </div>
        <Meter value={u.usedToday} max={u.cap} label={t(locale, 'aiUsageTitle')} />
      </Card>

      <Card title={t(locale, 'agentsTitle')}>
        {agents.data.items.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {agents.data.items.map((a) => (
              <article key={a.name} className="aur-tile">
                <span className="aur-tile__label" dir="ltr">
                  {a.emoji} {a.name}
                </span>
                <span className="aur-tile__value">{locale === 'fa' ? a.titleFa : a.titleEn}</span>
                <span className="text-aur-muted text-xs">{a.description}</span>
                <span className="aur-tile__sub" dir="ltr">
                  {a.skills.join(' · ')}
                </span>
              </article>
            ))}
          </div>
        )}
      </Card>

      <Card title={t(locale, 'skillsTitle')}>
        {skills.data.items.length === 0 ? (
          <EmptyState locale={locale} />
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table className="aur-table">
              <thead>
                <tr>
                  <th scope="col">{t(locale, 'colId')}</th>
                  <th scope="col">{t(locale, 'colType')}</th>
                  <th scope="col">{t(locale, 'colName')}</th>
                  <th scope="col">{t(locale, 'allowedAgents')}</th>
                </tr>
              </thead>
              <tbody>
                {skills.data.items.map((s) => (
                  <tr key={s.id}>
                    <td dir="ltr">
                      <code className="text-xs">{s.id}</code>
                    </td>
                    <td>
                      <span className={`aur-badge ${s.kind === 'money' ? 'aur-badge--warning' : ''}`}>
                        {skillKindLabel(locale, s.kind)}
                      </span>
                      {s.staffOnly && (
                        <span className="aur-badge aur-badge--info">{t(locale, 'staffOnlyBadge')}</span>
                      )}
                    </td>
                    <td>
                      {locale === 'fa' ? s.titleFa : s.titleEn}
                      <div className="text-aur-faint text-xs">{s.description}</div>
                    </td>
                    <td dir="ltr">
                      {s.allowedAgents.length === 0 ? (
                        <span className="aur-badge aur-badge--danger">{t(locale, 'noAgent')}</span>
                      ) : (
                        <span className="text-xs">{s.allowedAgents.join(', ')}</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </div>
  );
}
