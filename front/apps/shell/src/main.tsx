import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import App from './App.js';
import { registerServiceWorker } from './pwa.js';
import './shell.css';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // The shell is a strategic index — never fabricate state while offline.
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </StrictMode>,
);

// PWA (Step 6): prod-only, capability-guarded — a failed registration must
// never break the shell, so it degrades honestly with a warning.
void registerServiceWorker().catch((e) => {
  console.warn('[shell] service worker registration skipped:', e);
});
