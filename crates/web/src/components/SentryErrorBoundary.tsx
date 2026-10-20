import * as React from 'react';
import * as Sentry from '@sentry/react';
import { ComponentStack } from '@sentry/tracing';

interface SentryErrorBoundaryProps {
  fallback?: React.ReactNode;
  reset?: boolean;
}

interface SentryErrorState {
  hasError: boolean;
  error: unknown;
  errorInfo: unknown;
  resetErrorBoundary: () => void;
}

/**
 * Sentry Error Boundary - Captures rendering errors and reports them to Sentry
 * while providing a graceful fallback UI
 */
export class SentryErrorBoundary extends React.Component<
  SentryErrorBoundaryProps,
  SentryErrorState
> {
  constructor(props: SentryErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: unknown) {
    // Report the error to Sentry
    Sentry.captureException(error, {
      tags: {
        boundary: 'SentryErrorBoundary',
      },
      level: 'error',
    });

    return { hasError: true, error, errorInfo: null };
  }

  componentDidCatch(error: unknown, errorInfo: React.ErrorInfo) {
    // Additional context for Sentry
    Sentry.setExtra('error-boundary', 'SentryErrorBoundary');
    Sentry.setTag('component', this.constructor?.name || 'Unknown');
    Sentry.setTag('error-type', error instanceof Error ? error.constructor.name : 'Unknown');

    // Capture with full stack trace
    Sentry.captureException(error, {
      error: {
        message: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined,
      },
      tags: {
        boundary: 'SentryErrorBoundary',
        component: this.constructor?.name || 'Unknown',
      },
      level: 'error',
    });
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback || (
        <div className="sentry-error-fallback p-6 bg-red-50 rounded-lg">
          <h2 className="text-lg font-medium text-red-800 mb-2">
            Something went wrong
          </h2>
          <p className="text-red-600 small">
            We've been notified and are looking into it. Please try again.
          </p>
        </div>
      );
    }
    return this.props.children;
  }
}

/**
 * Wrap components with Sentry error boundary
 * Usage: <SentryErrorBoundary>...</SentryErrorBoundary>
 */
export const withSentryErrorBoundary = <
  T extends { new (...args: any[]): React.ComponentType<any> }
>(Component: T) => class extends React.Component {
  // ... can be used as HOC
};