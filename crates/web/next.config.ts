import { withSentryConfig } from '@sentry/nextjs/config';
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
    // The type check is `tsrs`, and it runs before the build -- in CI, in
    // the job that gates this one, and on a developer's machine with
    // `bun run type-check`.
    //
    // The build does not type-check a second time with a second compiler.
    // There is no second compiler here: the estate has one, and it is
    // `tsrs`. A build that checked the types again would be a second
    // opinion on the same tree, and the two could disagree.
    ignoreBuildErrors: process.env.SKIP_BUILD_TYPECHECK === "true",
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
