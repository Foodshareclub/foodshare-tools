# foodshare-api-client

The HTTP client the tools use to talk to the Foodshare backend.

Every call the tools make to the backend — the BFF, the Supabase Edge
Functions, the health endpoints — goes through here, so the resilience
behaviour is written once rather than in each caller.

## What it provides

- **Environment-based configuration** — URLs and keys come from the
  environment, so nothing has to be passed around or hard-coded.
- **Retry with exponential backoff** — a transient failure is retried
  rather than reported.
- **Circuit breaker** — an outage does not turn into a cascade of
  failing calls.
- **Rate limiting** — the client stays inside the API's limits.
- **Request correlation** — every request carries an id, so a call can
  be followed through the logs on both sides.

## Usage

```rust,no_run
use foodshare_api_client::{ClientConfig, FoodshareClient};

let client = FoodshareClient::new();
let health = client.health().check().await?;
```

## The endpoints

`crates/api-client/src/endpoints/` holds one module per surface —
`bff`, `health`, `localization`, `products`, `search`, `translations` —
each with the request and response types for that endpoint. The
middleware in `crates/api-client/src/middleware/` is where the retry,
the breaker and the correlation id are applied.
