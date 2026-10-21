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

## The toolchain is pinned

The generated modules are **committed to the web repository**. That makes
the toolchain that builds them part of the contract: a different one
produces different modules, and the difference lands in another
repository as a diff nobody asked for.

So the versions are pinned, and the build says so rather than assuming:

| What | Pinned in | Why it matters |
| --- | --- | --- |
| The dependency versions | `Cargo.lock`, which is committed | The modules embed the `wasm-bindgen` version, and every version renames the generated shims |
| `wasm-pack` | `mise.toml`, and `build::WASM_PACK_VERSION` | A different `wasm-pack` emits a different loader |

`fs-wasm` checks the `wasm-pack` version before it writes anything, and
stops with an explanation when it does not match:

```
❌ [foodshare-geo] failed: wasm-pack 0.15.0 is installed, but the
modules are built with 0.13.1; the two produce different modules, and
the modules are committed to the web repository
```

Nothing is written when the check fails, so the modules the web
repository tracks are never overwritten by a build that would change
them.

The version that built a module is recorded inside it — the generated
loader opens with:

```js
// Built by wasm-pack 0.13.1. Do not edit: this file is generated.
```

so a diff in the modules says which toolchain made it.

To move to a different `wasm-pack`, change `WASM_PACK_VERSION` and
`mise.toml` together, rebuild, and commit the modules with the change.

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
