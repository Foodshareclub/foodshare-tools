'use client';

import * as React from 'react';
import * as Sentry from '@sentry/react';

/**
 * Replay Button - Allows users to send a replay to Sentry
 * This helps the team understand frontend issues
 */
export function ReplayButton(): React.JSX.Element {
  const handleClick = () => {
    // Sentry 10 removed `showReportDialog`. `captureFeedback` is the
    // supported way to file a user report, and `includeReplay` attaches the
    // session replay that was buffered by the Replay integration.
    Sentry.captureFeedback(
      { message: 'User requested a replay of the last error' },
      { includeReplay: true },
    );
  };

  return (
    <button
      onClick={handleClick}
      className="sentry-replay-btn inline-flex items-center gap-2 px-4 py-2 border border-gray-300 rounded-md bg-white text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2"
      aria-label="Send error replay to Sentry"
      title="Send error replay"
    >
      <svg
        className="h-4 w-4"
        viewBox="0 0 24 24"
        fill="currentColor"
      >
        <path d="M19 9l-7 7-3-3-5 5L7 15l-5 5 5 5-3 3L7 21l5-5-5-5L7 9l9 7 7-9z" />
      </svg>
      <span>Send Replay</span>
    </button>
  );
}