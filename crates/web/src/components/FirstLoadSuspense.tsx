import * as React from 'react';

/**
 * FirstLoadSuspense - Shows a skeleton during initial load
 * Ensures static shell commits immediately for instant navigation
 * Critical for PPR (Partial Prerendering) and Cache Components
 */
export function FirstLoadSuspense({
  children,
  fallback,
}: {
  children: React.ReactElement;
  fallback?: React.ReactElement;
}) {
  // This component works with Next.js Cache Components / PPR
  // It ensures the static shell is served immediately
  
  return (
    <React.Suspense fallback={fallback || (
      <div className="first-load-skeleton p-8 text-center">
        <div className="animate-pulse h-6 w-24 mx-auto mb-4" />
        <p className="text-muted-foreground">Loading...</p>
      </div>
    )}>
      {children}
    </React.Suspense>
  );
}

/**
 * Export type for type-safety
 */
export type FirstLoadSuspenseProps = {
  children: React.ReactElement;
  fallback?: React.ReactElement;
};