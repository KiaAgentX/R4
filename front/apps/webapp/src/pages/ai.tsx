/**
 * Pages 22–23 — AI assistant chat + agents & skills directory
 * (map §2.4 rows 22–23). The chat is the citizen's ACTION surface (D14):
 * `POST /ai/chat` through the generated client with the RAW initData attached
 * (server-verified HMAC — identity is never faked client-side). History lives
 * in page memory only. The directory is a pure public read.
 */
import { useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { unwrap } from '@agent-one/api-client/errors';
import { t } from '@agent-one/i18n/webapp';
import type { PageCtx } from '../ctx.js';
import { userAccount } from '../session.js';
import { Card, EmptyState, ErrorState, Field, Kv, PendingState } from '../ui.js';

type ChatRole = 'user' | 'assistant';
interface ChatTurn {
  role: ChatRole;
  content: string;
  meta?: string | undefined;
  pending?: boolean | undefined;
}

const AGENT_NAMES = ['orchestrator', 'receptionist', 'sales', 'support', 'analyst', 'content'] as const;

function viaLabel(locale: PageCtx['locale'], via: string): string {
  if (via === 'direct') return t(locale, 'aiViaDirect');
  if (via === 'prefilter') return t(locale, 'aiViaPrefilter');
  return t(locale, 'aiViaLlm');
}

export default function AiChatPage({ ctx, initialPrefill }: { ctx: PageCtx; initialPrefill?: string | undefined }) {
  const { client, locale, mode, initData, tgUser } = ctx;
  const [turns, setTurns] = useState<ChatTurn[]>([]);
  const [input, setInput] = useState(initialPrefill ?? '');
  const [userId, setUserId] = useState(tgUser !== null ? userAccount(tgUser.id) : '');
  const [directAgent, setDirectAgent] = useState<string>('');
  const sending = useRef(false);
  const staff = mode.kind === 'staff';
  const citizen = initData !== null;

  async function send(): Promise<void> {
    const text = input.trim();
    if (text.length === 0 || sending.current) return;
    sending.current = true;
    const history = turns.filter((x) => !x.pending).slice(-6).map((x) => ({ role: x.role, content: x.content }));
    setTurns((prev) => [...prev, { role: 'user', content: text }, { role: 'assistant', content: '', pending: true }]);
    setInput('');
    try {
      const res = await client.ai.chat({
        userId: userId.trim(),
        text,
        language: locale,
        ...(directAgent !== '' ? { directAgent: directAgent as (typeof AGENT_NAMES)[number] } : {}),
        ...(history.length > 0 ? { history } : {}),
        // The raw initData goes through the identity layer of the client.
        ...(initData !== null ? { initData } : {}),
        ...(staff ? { role: 'staff' as const } : {}),
      });
      const data = await unwrap(res);
      const metaBits = [data.agent, viaLabel(locale, data.via)];
      if (data.usage !== null && data.usage !== undefined && data.usage.totalTokens > 0) {
        metaBits.push(`${data.usage.totalTokens} tok`);
      }
      const pending = data.toolTraces.filter((x) => x.pendingApproval).length;
      const traces = data.toolTraces
        .map((x) => `${x.ok ? '✓' : '✗'} ${x.skill}: ${x.summary}${x.pendingApproval ? ` · ${t(locale, 'aiPendingTool')}` : ''}`)
        .join('\n');
      setTurns((prev) => [
        ...prev.slice(0, -1),
        {
          role: 'assistant',
          content: `${data.reply}${traces.length > 0 ? `\n— ${t(locale, 'aiToolTraces')} —\n${traces}` : ''}`,
          meta: `${metaBits.join(' · ')}${pending > 0 ? ` · ${pending} ${t(locale, 'aiPendingTool')}` : ''}`,
        },
      ]);
    } catch (e) {
      setTurns((prev) => [...prev.slice(0, -1), { role: 'assistant', content: t(locale, 'actionFailed'), meta: String((e as Error).message ?? '') }]);
    } finally {
      sending.current = false;
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <Card
        title={t(locale, 'aiChatTitle')}
        note={t(locale, 'aiHistoryNote')}
      >
        {staff ? (
          <div className="aur-badge aur-badge--brand">{t(locale, 'aiStaffMode')}</div>
        ) : citizen ? (
          <div className="aur-badge aur-badge--brand">{t(locale, 'aiIdentityOk')}</div>
        ) : (
          <div role="note" className="text-sm text-aur-muted">
            {t(locale, 'aiIdentityNote')}
          </div>
        )}
        {staff && (
          <Field id="w-ai-user" label={t(locale, 'aiUserIdLabel')} hint={t(locale, 'buySellerIdHint')}>
            <input id="w-ai-user" className="aur-input" dir="ltr" value={userId} onChange={(e) => setUserId(e.target.value)} />
          </Field>
        )}
        <Field id="w-ai-agent" label={t(locale, 'aiAgentSelector')}>
          <select id="w-ai-agent" className="aur-input" value={directAgent} onChange={(e) => setDirectAgent(e.target.value)}>
            <option value="">—</option>
            {AGENT_NAMES.map((a) => (
              <option key={a} value={a}>
                {a}
              </option>
            ))}
          </select>
        </Field>
      </Card>

      <Card>
        <div className="aur-chat" aria-live="polite">
          {turns.length === 0 && <EmptyState locale={locale} note={t(locale, 'aiChatHint')} />}
          {turns.map((turn, i) => (
            <div key={i} className={`aur-chat__msg aur-chat__msg--${turn.role === 'user' ? 'user' : 'bot'}`} data-pending={turn.pending === true ? 'true' : undefined}>
              <strong className="text-xs">{turn.role === 'user' ? t(locale, 'aiYou') : t(locale, 'aiAssistant')}</strong>
              <div>{turn.pending === true ? t(locale, 'loading') : turn.content}</div>
              {turn.meta !== undefined && turn.meta !== '' && <div className="aur-chat__meta">{turn.meta}</div>}
            </div>
          ))}
        </div>
        <div className="flex gap-2">
          <input
            className="aur-input"
            aria-label={t(locale, 'aiChatPlaceholder')}
            value={input}
            placeholder={t(locale, 'aiChatHint')}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void send();
            }}
          />
          <button type="button" className="aur-btn aur-btn--primary" disabled={input.trim().length === 0} onClick={() => void send()}>
            {t(locale, 'aiChatSend')}
          </button>
        </div>
        <button
          type="button"
          className="aur-btn aur-btn--ghost aur-btn--sm"
          onClick={() => setTurns([])}
          disabled={turns.length === 0}
        >
          {t(locale, 'aiClear')}
        </button>
      </Card>
    </div>
  );
}

/* ── Page 23 — agents & skills directory ──────────────────────────────── */

const SKILL_KIND = {
  read: 'skillKindRead',
  write: 'skillKindWrite',
  money: 'skillKindMoney',
  staff: 'skillKindStaff',
} as const;

export function AiDirectoryPage({ ctx }: { ctx: PageCtx }) {
  const { client, locale } = ctx;
  const agents = useQuery({ queryKey: ['w-agents'], queryFn: async () => unwrap(await client.reads.agents()), staleTime: 60_000 });
  const skills = useQuery({ queryKey: ['w-skills'], queryFn: async () => unwrap(await client.reads.skills()), staleTime: 60_000 });

  if (agents.isPending || skills.isPending) return <PendingState locale={locale} />;
  if (agents.isError) return <ErrorState locale={locale} error={agents.error} onRetry={() => void agents.refetch()} />;
  if (skills.isError) return <ErrorState locale={locale} error={skills.error} onRetry={() => void skills.refetch()} />;

  return (
    <div className="flex flex-col gap-4">
      <Card title={t(locale, 'agentsTitle')}>
        <div className="flex flex-col gap-3">
          {agents.data.items.map((a) => (
            <div key={a.name} className="aur-receipt">
              <span className="aur-receipt__head">
                <span>
                  {a.emoji} {a.titleFa} · {a.titleEn}
                </span>
                <code className="text-xs" dir="ltr">
                  {a.name}
                </code>
              </span>
              <span className="text-sm text-aur-muted">{a.description}</span>
              <span className="text-xs text-aur-faint" dir="ltr">
                {a.skills.length > 0 ? a.skills.join(' · ') : t(locale, 'noAgent')}
              </span>
            </div>
          ))}
        </div>
      </Card>
      <Card title={t(locale, 'skillsTitle')}>
        <div className="flex flex-col gap-3">
          {skills.data.items.map((s) => (
            <div key={s.id} className="aur-receipt">
              <span className="aur-receipt__head">
                <span>
                  <span className="aur-badge">{t(locale, SKILL_KIND[s.kind])}</span>{' '}
                  {s.titleFa} · {s.titleEn}
                </span>
                {s.staffOnly && <span className="aur-badge aur-badge--warning">{t(locale, 'staffOnlyBadge')}</span>}
              </span>
              <span className="text-sm text-aur-muted">{s.description}</span>
              <Kv label={t(locale, 'allowedAgents')} ltr>
                <code className="text-xs">{s.allowedAgents.length > 0 ? s.allowedAgents.join(' · ') : t(locale, 'noAgent')}</code>
              </Kv>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}
