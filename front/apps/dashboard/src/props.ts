/**
 * Props every dashboard section receives from the App frame — the assembled
 * typed client (admin key already applied) and the active locale.
 */
import type { AgentOneClient } from '@agent-one/api-client';
import type { Locale } from '@agent-one/i18n/dashboard';

export interface SectionProps {
  client: AgentOneClient;
  locale: Locale;
}
