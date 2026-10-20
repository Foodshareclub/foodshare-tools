import './globals.css';
import { SentryErrorBoundary } from '@/components/SentryErrorBoundary';
import { FeedbackWidget } from '@/components/FeedbackWidget';
import { ReplayButton } from '@/components/ReplayButton';
import { FirstLoadSuspense } from '@/components/FirstLoadSuspense';

export const metadata = {
  title: 'FoodShare - Food Sharing Platform',
  description: 'Connect surplus food with those in need. Reduce waste, feed communities.',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>
        <FirstLoadSuspense>
          <SentryErrorBoundary>
            <FeedbackWidget />
            <ReplayButton />
            {children}
          </SentryErrorBoundary>
        </FirstLoadSuspense>
      </body>
    </html>
  );
}