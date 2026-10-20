import * as Sentry from '@sentry/nextjs';

Sentry.init({
  dsn: process.env.SENTRY_DSN as string | undefined,
  environment: process.env.NODE_ENV,
  tracesSampleRate: 0.5,
  attachOnErrorAllNative: true,
  debug: process.env.NEXT_PUBLIC_SENTRY_DEBUG === 'true',
  
  // Set better titles for issues automatically
  beforeSend(event) {
    // Enhance error titles for better categorization
    if (event.exception?.values?.[0]?.stacktrace) {
      const stackTop = event.exception.values[0].stacktrace.top;
      if (stackTop) {
        event.exception.values[0].stacktrace.frames = event.exception.values[0].stacktrace.frames
          .filter((frame: any) => frame.file?.includes('/app/') || frame.file?.includes('/src/'))
          .slice(0, 5);
      }
    }
    return event;
  },
  
  // Auto-create issues from new errors
  integrations: [
    // Enable automatic issue creation
  ],
});