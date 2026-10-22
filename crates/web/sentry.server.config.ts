import * as Sentry from '@sentry/nextjs';

Sentry.init({
  dsn: process.env.SENTRY_DSN as string | undefined,
  environment: process.env.NODE_ENV,
  tracesSampleRate: 0.5,
  debug: process.env.NEXT_PUBLIC_SENTRY_DEBUG === 'true',

  // Keep only the frames from the application. The deployment's own stack is
  // what a reader needs, and the framework's frames bury it.
  //
  // Sentry 8 removed `stacktrace.top` — the topmost frame is `frames[0]` —
  // and the frames are optional, so the check has to be for the frames
  // rather than for a frame that is no longer part of the type.
  beforeSend(event) {
    const exception = event.exception?.values?.[0];
    const stacktrace = exception?.stacktrace;
    const frames = stacktrace?.frames;
    if (stacktrace && frames?.length) {
      stacktrace.frames = frames
        .filter((frame) => frame.filename?.includes('/app/') || frame.filename?.includes('/src/'))
        .slice(0, 5);
    }
    return event;
  },
});
