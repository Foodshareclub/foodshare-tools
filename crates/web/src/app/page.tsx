import Image from 'next/image';
import Link from 'next/link';

// `cacheComponents` replaces the route-segment `revalidate` and `dynamic`
// exports, which Next 16 rejects outright. This page reads no data and
// uses no dynamic APIs, so it is prerendered as static content by default.
export default function HomePage() {
  return (
    <main className="min-h-screen bg-background p-6">
      <header className="mb-8">
        <h1 className="text-4xl font-bold text-primary">
          FoodShare
        </h1>
        <p className="text-lg text-muted-foreground">
          Connect surplus food with those in need
        </p>
      </header>

      <section className="grid grid-cols-1 gap-6 md:grid-cols-2">
        <div>
          <Image
            src="/foodshare-logo.svg"
            alt="FoodShare logo"
            width={200}
            height={200}
            className="rounded-xl"
          />
          <p className="mt-4 text-center text-sm text-muted-foreground">
            Reduce waste. Feed communities.
          </p>
        </div>

        <div>
          <h2 className="text-xl font-medium mb-4">Get Started</h2>
          <Link
            href="/login"
            className="inline-block bg-primary text-white px-4 py-2 rounded-md hover:bg-primary/90 transition-colors"
          >
            Sign In
          </Link>
          <Link
            href="/signup"
            className="inline-block mt-2 bg-secondary text-white px-4 py-2 rounded-md hover:bg-secondary/90 transition-colors"
          >
            Sign Up
          </Link>
        </div>
      </section>
    </main>
  );
}