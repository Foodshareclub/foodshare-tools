import * as Sentry from '@sentry/react';
import { Replay } from '@sentry/replay';
import { feedbackModalIntegration } from '@sentry/feedback';

Sentry.init({
  dsn: process.env.NEXT_PUBLIC_SENTRY_DSN as string,
  environment: process.env.NODE_ENV,
  integrations: [
    new Replay({
      maskAll: true,
      blockAll: true,
      // Only record when user hasn't disabled it
      maskPII: true,
    }),
    // Sentry 10 split the feedback integration: the modal now lives in
    // `@sentry/feedback`, and `stopJavascriptExecutionIntegration` is folded
    // into it rather than being configured separately.
    feedbackModalIntegration(),
  ],
  tracesSampleRate: 0.5,
  replaysSampleRate: 0.3,

  // Performance monitoring
  defaultIntegrations: [Sentry.browserTracingIntegration()],
});
