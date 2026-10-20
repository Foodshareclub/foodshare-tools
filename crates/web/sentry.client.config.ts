import * as Sentry from '@sentry/react';
import { Replay } from '@sentry/replay';

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
    // Feedback integration for issue reporting
    Sentry.feedbackIntegration({
      color: '#1a73e8',
      emoji: '🐛',
      characterCounterMax: 500,
    }),
  ],
  tracesSampleRate: 0.5,
  replaysSampleRate: 0.3,
  
  // Performance monitoring
  enableTracing: true,
  
  // Better error grouping
  default integrations: [
    Sentry.browserTracingIntegration(),
    Sentry.stopJavascriptExecutionIntegration(),
  ],
});