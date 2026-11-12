import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import App from './App.js';
import './webapp.css';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // Citizen views never fabricate state while offline — honest pending/error.
      retry: 1,
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: 0, // idempotency keys protect replays; blind mutation retries do not
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
