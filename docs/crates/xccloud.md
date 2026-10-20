# foodshare-xccloud

Xcode Cloud build runs, logs and diagnostics over the App Store Connect
API.

The tool queries the CI products of the team, their build runs, the
actions of a run, and the issues and artefacts an action produced. When
the credentials are not available it falls back to the local build
summaries CI leaves behind.

## Credentials

The credentials come from the environment:

- `APP_STORE_CONNECT_KEY_ID`, or `ASC_KEY_ID`
- `APP_STORE_CONNECT_ISSUER_ID`, or `ASC_ISSUER_ID`
- `APP_STORE_CONNECT_PRIVATE_KEY`, or `ASC_PRIVATE_KEY`

Otherwise the key file is looked for, under the known key identifiers and
then any `.p8` file, in:

- the app repository's `keys/` directory
- `~/.appstoreconnect/private_keys/`
- the app repository itself

Both `AuthKey_<KEY_ID>.p8` and `<KEY_ID>.p8` are accepted.

## The token

The API authenticates with a JSON Web Token signed with the private key,
in the ES256 form: the signature is the raw `r || s` pair, not the DER
encoding. The token is valid for twenty minutes.

```rust,no_run
use foodshare_xccloud::auth;

let locations = auth::default_locations(std::path::Path::new("foodshare-app"));
let credentials = auth::resolve(&locations).expect("credentials");

let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs() as i64)
    .unwrap_or(0);
let token = auth::create_token(&credentials, now).expect("token");
```

## The client

```rust,no_run
use foodshare_xccloud::api::Client;

let client = Client::new(&credentials)?;

// The CI products of the team.
for product in client.products().await?.data {
    println!("{}", product.attributes.name.unwrap_or_default());
}

// The build runs of one product, newest first.
let runs = client.build_runs(&product.id, 5).await?;
```

## The local summaries

CI writes a Markdown summary of each build into the app repository's
`logs/xcodecloud/` directory. They are what a developer reads when the
API is not reachable.

```rust,no_run
use foodshare_xccloud::diagnostics;

for (path, contents) in diagnostics::read_summaries(app_dir, 3, 1500) {
    println!("--- {} ---\\n{contents}", path.display());
}
```
