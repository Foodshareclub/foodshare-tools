import * as Sentry from '@sentry/react';
import { replayIntegration } from '@sentry/replay';
import { feedbackModalIntegration } from '@sentry/feedback';

Sentry.init({
  dsn: process.env.NEXT_PUBLIC_SENTRY_DSN as string,
  environment: process.env.NODE_ENV,
  integrations: [
    // Sentry 10 renamed the replay integration and its options: the `Replay`
    // class became `replayIntegration`, and `maskAll`, `blockAll` and
    // `maskPII` became `maskAllText`, `blockAllMedia` and `maskAllInputs`.
    //
    // The old names were of things that no longer exist, so the browser would
    // have thrown on `new Replay(...)` the first time it ran this file.
    replayIntegration({
      maskAllText: true,
      maskAllInputs: true,
      blockAllMedia: true,
    }),
    // Sentry 10 split the feedback integration: the modal now lives in
    // `@sentry/feedback`, and `stopJavascriptExecutionIntegration` is folded
    // into it rather than being configured separately.
    feedbackModalIntegration(),
  ],
  tracesSampleRate: 0.5,
  // Sentry 10 split the replay sample rate in two: how many sessions are
  // recorded, and how many of the sessions that had an error are.
  replaysSessionSampleRate: 0.3,
  replaysOnErrorSampleRate: 1.0,

  // Performance monitoring
  defaultIntegrations: [Sentry.browserTracingIntegration()],
});
