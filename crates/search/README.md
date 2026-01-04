# foodshare-search

High-performance fuzzy search and text matching utilities.

[![Crates.io](https://img.shields.io/crates/v/foodshare-search.svg)](https://crates.io/crates/foodshare-search)
[![Documentation](https://docs.rs/foodshare-search/badge.svg)](https://docs.rs/foodshare-search)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Relevance Scoring** - Multi-level scoring (exact, starts-with, contains, fuzzy)
- **Fuzzy Matching** - Find matches even with typos
- **Levenshtein Distance** - Calculate edit distance between strings
- **Unicode Support** - Proper handling of international characters
- **WASM Support** - Compile to WebAssembly for browser usage

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
foodshare-search = "1.3"
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `parallel` | Yes | Enable rayon for parallel processing |
| `wasm` | No | Enable WebAssembly bindings |

## Usage

### Relevance Scoring

Calculate how well a query matches text:

```rust
use foodshare_search::calculate_relevance;

// Returns score from 0-50 (higher is better)
let score = calculate_relevance("Hello World", "hello");
// score = 40 (StartsWith)

let score = calculate_relevance("Say Hello", "hello");
// score = 30 (WordBoundary)

let score = calculate_relevance("Hello", "hello");
// score = 50 (Exact match)
```

### Score Levels

| Score | Match Type | Example (query: "hello") |
|-------|------------|--------------------------|
| 50 | Exact | "Hello" |
| 40 | Starts With | "Hello World" |
| 30 | Word Boundary | "Say Hello" |
| 20 | Contains | "SayHelloWorld" |
| 10 | Fuzzy | "H...e...l...l...o" |
| 0 | No Match | "Goodbye" |

### Fuzzy Matching

Check if all query characters appear in text in order:

```rust
use foodshare_search::fuzzy_match;

assert!(fuzzy_match("Hello World", "hwo"));  // h...w...o
assert!(fuzzy_match("Hello", "hel"));        // true
assert!(!fuzzy_match("Hello", "leh"));       // wrong order
```

### Levenshtein Distance

Calculate the number of edits to transform one string into another:

```rust
use foodshare_search::levenshtein_distance;

let dist = levenshtein_distance("hello", "hallo");  // 1 (substitute)
let dist = levenshtein_distance("hello", "helo");   // 1 (delete)
let dist = levenshtein_distance("cat", "dog");      // 3 (all different)
```

### Search and Rank Results

```rust
use foodshare_search::calculate_relevance;

struct Product {
    id: String,
    name: String,
}

fn search_products(query: &str, products: &[Product]) -> Vec<(&Product, u32)> {
    let mut results: Vec<_> = products
        .iter()
        .map(|p| (p, calculate_relevance(&p.name, query)))
        .filter(|(_, score)| *score > 0)
        .collect();
    
    results.sort_by(|a, b| b.1.cmp(&a.1));
    results
}
```

## WASM Usage

For browser usage, see [@foodshare/search-wasm](https://www.npmjs.com/package/@foodshare/search-wasm).

```typescript
import init, { relevance_score, search_items } from '@foodshare/search-wasm';

await init();

// Single score
const score = relevance_score("hello", "Hello World");

// Batch search
const items = JSON.stringify([
  { id: "1", text: "Fresh Apples" },
  { id: "2", text: "Apple Pie" },
]);
const results = JSON.parse(search_items("apple", items, 10));
```

## Performance

- ~10x faster than JavaScript implementations
- Efficient memory usage with optimized algorithms
- Optional parallel processing with rayon

## License

MIT License - see [LICENSE](LICENSE) for details.
