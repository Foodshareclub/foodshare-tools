# foodshare-wasm

WebAssembly builds for the published Foodshare crates.

The five crates that ship to the browser are compiled with `wasm-pack`,
and the generated loaders are patched so Next.js can find the `.wasm`
files after bundling.

## The crates

| Crate                    | What it ships                        |
| ------------------------ | ------------------------------------ |
| `foodshare-geo`          | Geospatial calculations              |
| `foodshare-search`       | Vector search and fuzzy matching     |
| `foodshare-crypto`       | TOTP and HMAC                        |
| `foodshare-compression`  | Brotli and gzip                      |
| `foodshare-image`        | Format detection                     |

## The target

The `nodejs` target is deliberate: the bridges in the web app are
server-only, so they load the module synchronously. Switching to
`bundler` or `web` would mean making those bridges async first.

## The loader patch

`wasm-pack` writes a loader that resolves the `.wasm` file relative to
`__dirname`. Next.js bundles the module, so `__dirname` is no longer the
package directory and the loader cannot find the file. The generated
JavaScript is patched to try a list of candidate paths instead.

A file that does not hold the expected loader is left alone: a newer
`wasm-pack` may write something else, and rewriting it blindly would
break the build.

## Usage

```rust,no_run
use foodshare_wasm::{build, loader, Layout};

let layout = Layout {
    tools_dir: "foodshare-tools".into(),
    web_dir: "foodshare-web".into(),
};

let outcomes = build::build_all(&layout, loader::CRATES).await;
```

The builds run at the same time: they share the Cargo target directory,
and running them one after another would serialise the whole dependency
compile.
