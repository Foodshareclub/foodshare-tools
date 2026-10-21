//! The Turbopack path resolver.
//!
//! `wasm-pack` writes a loader that resolves the `.wasm` file relative to
//! `__dirname`. Next.js bundles the module, so `__dirname` is no longer
//! the package directory and the loader cannot find the file. The
//! generated JavaScript is patched to try a list of candidate paths
//! instead.

/// The crate the loader belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crate {
    /// The crate's package name, such as `foodshare-geo`.
    pub name: &'static str,
    /// The crate's directory in the Rust workspace.
    pub path: &'static str,
}

impl Crate {
    /// The name of the generated JavaScript file, without its extension.
    #[must_use]
    pub fn js_name(&self) -> String {
        self.name.replace('-', "_")
    }

    /// The name of the generated WebAssembly file.
    #[must_use]
    pub fn wasm_name(&self) -> String {
        format!("{}_bg.wasm", self.js_name())
    }
}

/// The crates that ship as WebAssembly.
pub const CRATES: &[Crate] = &[
    Crate {
        name: "foodshare-geo",
        path: "crates/geo",
    },
    Crate {
        name: "foodshare-search",
        path: "crates/search",
    },
    Crate {
        name: "foodshare-crypto",
        path: "crates/crypto",
    },
    Crate {
        name: "foodshare-compression",
        path: "crates/compression",
    },
    Crate {
        name: "foodshare-image",
        path: "crates/image",
    },
];

/// The loader `wasm-pack` writes, which cannot find the file once the
/// module is bundled.
fn old_loader(wasm_name: &str) -> String {
    format!(
        "const wasmPath = `${{__dirname}}/{wasm_name}`;\nconst wasmBytes = require('fs').readFileSync(wasmPath);"
    )
}

/// The loader that replaces it.
fn universal_loader(crate_name: &str, wasm_name: &str) -> String {
    format!(
        r#"
function __resolveWasmPath(filename) {{
    const fs = require('fs');
    const path = require('path');
    const candidates = [
        path.join(__dirname, filename),
        path.join(process.cwd(), "src", "wasm", "{crate_name}", filename),
        path.join(process.cwd(), "foodshare-web", "src", "wasm", "{crate_name}", filename),
        path.join(process.cwd(), "..", "foodshare-web", "src", "wasm", "{crate_name}", filename),
    ];
    for (const c of candidates) {{
        if (fs.existsSync(/*turbopackIgnore: true*/ c)) return c;
    }}
    return candidates[0];
}}
const wasmPath = __resolveWasmPath('{wasm_name}');
const wasmBytes = require('fs').readFileSync(/*turbopackIgnore: true*/ wasmPath);"#
    )
}

/// The header a patched loader carries.
///
/// The modules are committed to the web repository, so a diff in one of
/// them should say what built it. Without this, a change in the toolchain
/// looks like a set of changed hashes to be interpreted.
fn toolchain_header() -> String {
    format!(
        "// Built by wasm-pack {}. Do not edit: this file is generated.\n",
        crate::build::WASM_PACK_VERSION
    )
}

/// Patch a generated JavaScript file so it can find its `.wasm` file.
///
/// Returns the patched source, or `None` when the file does not hold the
/// expected loader and was left alone: a newer `wasm-pack` may write
/// something else, and rewriting it blindly would break the build.
#[must_use]
pub fn patch_loader(source: &str, krate: &Crate) -> Option<String> {
    let wasm_name = krate.wasm_name();
    let old = old_loader(&wasm_name);
    if !source.contains(&old) {
        return None;
    }
    let new = universal_loader(krate.name, &wasm_name);
    let header = toolchain_header();
    // The header goes on once. The loader is patched on every build, and
    // the file it is patched in is the one wasm-pack just wrote.
    let without_header = source.strip_prefix(&header).unwrap_or(source);
    Some(format!(
        "{header}{}",
        without_header.replace(&old, new.trim())
    ))
}

/// The name of the generated JavaScript file for a crate.
#[must_use]
pub fn js_file_name(krate: &Crate) -> String {
    format!("{}.js", krate.js_name())
}

/// The `.gitignore` a vendored module directory needs.
///
/// `wasm-pack` writes a plain `*`, which would ignore the type
/// definitions and the package manifest that the web repository tracks
/// on purpose. The un-ignore lines are restored after every build.
pub const VENDORED_GITIGNORE: &str = "*\n!*.d.ts\n!package.json\n!.gitignore";

/// The `.gitignore` the vendored module directory should have.
#[must_use]
pub fn vendored_gitignore() -> String {
    VENDORED_GITIGNORE.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geo() -> Crate {
        CRATES[0]
    }

    #[test]
    fn test_names_are_underscored() {
        assert_eq!(geo().js_name(), "foodshare_geo");
        assert_eq!(geo().wasm_name(), "foodshare_geo_bg.wasm");
        assert_eq!(js_file_name(&geo()), "foodshare_geo.js");
    }

    #[test]
    fn test_patch_replaces_the_loader() {
        let source = format!(
            "const x = 1;\n{}\nconst y = 2;\n",
            old_loader("foodshare_geo_bg.wasm")
        );
        let patched = patch_loader(&source, &geo()).expect("patched");
        assert!(!patched.contains("const wasmPath = `${__dirname}"));
        assert!(patched.contains("__resolveWasmPath"));
        assert!(patched.contains("const x = 1;"));
    }

    #[test]
    fn test_patch_leaves_an_unknown_loader_alone() {
        let source = "something else entirely";
        assert!(patch_loader(source, &geo()).is_none());
    }

    #[test]
    fn test_the_patch_says_what_built_the_module() {
        let source = format!(
            "const x = 1;\n{}\nconst y = 2;\n",
            old_loader("foodshare_geo_bg.wasm")
        );
        let patched = patch_loader(&source, &geo()).expect("patched");
        assert!(patched.starts_with(&toolchain_header()), "got: {patched}");
        assert!(patched.contains(crate::build::WASM_PACK_VERSION));
    }

    #[test]
    fn test_the_header_is_added_once() {
        let source = format!(
            "const x = 1;\n{}\nconst y = 2;\n",
            old_loader("foodshare_geo_bg.wasm")
        );
        // A file that already carries the header — a re-patch — does not
        // end up with two of them.
        let once =
            patch_loader(&format!("{}{source}", toolchain_header()), &geo()).expect("patched");
        assert_eq!(
            once.matches("Built by wasm-pack").count(),
            1,
            "the header was added twice"
        );
    }

    #[test]
    fn test_a_patched_loader_is_not_patched_again() {
        let source = format!(
            "const x = 1;\n{}\nconst y = 2;\n",
            old_loader("foodshare_geo_bg.wasm")
        );
        let patched = patch_loader(&source, &geo()).expect("patched");
        // The loader `wasm-pack` writes is gone, so there is nothing left
        // to replace.
        assert!(patch_loader(&patched, &geo()).is_none());
    }

    #[test]
    fn test_the_universal_loader_mentions_every_candidate() {
        let loader = universal_loader("foodshare-geo", "foodshare_geo_bg.wasm");
        assert!(loader.contains("__dirname"));
        assert!(loader.contains("foodshare-web"));
        assert!(loader.contains("foodshare_geo_bg.wasm"));
        assert!(loader.contains("turbopackIgnore"));
    }

    #[test]
    fn test_the_vendored_gitignore_keeps_the_tracked_files() {
        let gitignore = vendored_gitignore();
        assert!(gitignore.starts_with("*\n"));
        assert!(gitignore.contains("!*.d.ts"));
        assert!(gitignore.contains("!package.json"));
        assert!(gitignore.contains("!.gitignore"));
    }
}
