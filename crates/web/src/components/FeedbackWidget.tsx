import * as React from 'react';
import * as Sentry from '@sentry/react';
import { feedbackIntegration } from '@sentry/feedback';

/**
 * Feedback Widget - Allows users to submit feedback on errors
 * Integrates with Sentry's feedback feature
 */
export function FeedbackWidget(): JSX.Element {
  const [showFeedback, setShowFeedback] = React.useState(false);
  const [name, setName] = React.useState('');
  const [email, setEmail] = React.useState('');
  const [message, setMessage] = React.useState('');
  const [sending, setSending] = React.useState(false);

  const handleSubmit = () => {
    if (!name.trim() || !email.trim() || !message.trim()) {
      return;
    }

    setSending(true);

    Sentry.getCurrentClient()?.showReportDialog({
      onSubmit: ({ name: submittedName, email: submittedEmail, comment }) => {
        // Send feedback to Sentry
        feedbackIntegration?.sendFeedback?.({
          name: submittedName,
          email: submittedEmail,
          comment,
        });
        setShowFeedback(false);
        setName('');
        setEmail('');
        setMessage('');
        setSending(false);
      },
    });
  };

  return (
    <div className="feedback-widget hidden" data-testid="feedback-widget">
      <button
        onClick={() => setShowFeedback(true)}
        className="feedback-trigger inline-flex items-center gap-2 px-4 py-2 border border-gray-300 rounded-md bg-white text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2"
        aria-label="Submit feedback for error"
        title="Submit feedback"
      >
        <svg
          className="h-4 w-4"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M19 9l-7 7-3-3-5 5L7 15l-5 5 5 5-3 3L7 21l5-5-5-5L7 9l9 7 7-9z" />
        </svg>
        <span>Send Feedback</span>
      </button>

      {/* Feedback modal */}
      {showFeedback && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-zxl z-50 flex items-center justify-center p-4">
          <div className="bg-white rounded-lg p-6 max-w-md w-full box-shadow-xl">
            <h3 className="text-xl font-bold mb-4">Send Feedback</h3>
            
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Your name
                </label>
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Your name"
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2"
                  required
                />
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Email
                </label>
                <input
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  type="email"
                  placeholder="you@example.com"
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2"
                  required
                />
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  What happened?
                </label>
                <textarea
                  value={message}
                  onChange={(e) => setMessage(e.target.value)}
                  placeholder="Describe what you were doing..."
                  rows={4}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 resize-y"
                  required
                ></textarea>
              </div>
            </div>
            
            <div className="flex gap-3 mt-6">
              <button
                onClick={() => setShowFeedback(false)}
                className="flex-1 px-4 py-2 border border-gray-300 rounded-md bg-gray-50 text-sm font-medium text-gray-700 hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
                type="button"
                aria-label="Cancel feedback"
              >
                Cancel
              </button>
              <button
                onClick={handleSubmit}
                disabled={sending}
                className="flex-1 px-4 py-2 bg-primary text-white rounded-md text-sm font-medium hover:bg-primary/90 focus:outline-none focus:ring-2 focus-ring-primary focus:ring-offset-2"
                type="button"
                aria-label="Send feedback"
              >
                {sending ? 'Sending...' : 'Send Feedback'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}