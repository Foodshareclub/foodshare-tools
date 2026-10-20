import { withSentryConfig } from '@sentry/nextjs';
import type { NextConfig } from 'next';

const nextConfig: NextConfig = {
  // Cache Components (PPR). Prerenders the static shell of a route and
  // streams the rest, which is what makes the shell commit instantly.
  //
  // In Next 16 these are top-level keys. `experimental.cacheComponents` is
  // deprecated and `experimental.partialPrerendering` was folded into
  // `cacheComponents` — both are rejected by the config validator now.
  cacheComponents: true,

  // Partial Prefetching: a `<Link>` prefetches only the static part of a
  // route, never its dynamic data. Requires `cacheComponents`.
  partialPrefetching: true,

  // Root domain configuration
  // domain: process.env.NEXT_PUBLIC_URL,

  // Output directory
  distDir: '.next',

  // Image configuration
  images: {
    remotePatterns: [
      {
        protocol: 'https',
        hostname: '**',
      },
    ],
  },

  // i18n configuration
  // i18n: {
  //   locales: ['en-US', 'en', 'es', 'fr'],
  //   defaultLocale: 'en-US',
  // },

  // webpack configuration for Sentry
  webpack: (config, { isServer }) => {
    // Sentry webpack plugin configuration
    if (!isServer) {
      // Fixes npm moment dependency conflict
      config.resolve.alias!.fsevents = false;
    }

    return config;
  },

  // TypeScript strict checks
  typescript: {
    // Type errors fail the build; `bun run type-check` runs the same check.
    ignoreBuildErrors: false,
  },
};

// Sentry configuration options — only in production
const sentryWebpackPluginOptions = {
  silent: true,
  org: process.env.SENTRY_ORG || 'organicnz-7v',
  project: process.env.SENTRY_PROJECT || 'foodshare-web',
  authToken: process.env.SENTRY_AUTH_TOKEN,
  tunnelRoute: '/monitoring',
};

export default withSentryConfig(nextConfig, sentryWebpackPluginOptions);
