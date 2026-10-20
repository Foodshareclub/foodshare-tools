/** @type {import('next').NextConfig} */
const nextConfig = {
  // Enable App Router and Cache Components ( bleeding edge )
  experimental: {
    // Cache Components - enables PPR (Partial Prerendering)
    cacheComponents: true,
    // Partial Prerendering - instant navigation
    partialPrerendering: true,
    // Actions in SSR
    actions: true,
    // new css composer
    newCssComposer: true,
  },
  
  // Sentry configuration
  sentry: {
    // Auth token for source map upload
    // tunnel: {
    //   tunnelUrl: process.env.SENTRY_TUNNEL_URL,
    // },
    
    // Include source maps for better error reporting
    // widerClientFileGlobs: ['**/*.{js,ts,jsx,tsx}'],
  },
  
  // Root domain configuration
  // domain: process.env.NEXT_PUBLIC_URL,
  
  // Output directory
  distDir: '.next',
  
  // Image configuration
  images: {
    domains: ['images.unsplash.com', 'picsum.photos'],
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
    // !! WARN !! -
    // Dangerously allow production builds successfully type-check if types are
    // incorrect. This is useful if you are seeing errors from 3rd party
    // components that you don't have control over.
    ignoreBuildErrors: false,
  },
  
  // ESLint configuration
  eslint: {
    // Run ESLint only on changed files in production
    ignoreDuringBuilds: false,
  },
};

export default nextConfig;