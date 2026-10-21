# foodshare-search

High-performance fuzzy text search.

## Installation

```toml
[dependencies]
foodshare-search = "1.4"
```

## Features

- Fuzzy string matching
- Ranked results
- Unicode support
- WASM support

## Usage

```rust
use foodshare_search::{fuzzy_match, levenshtein_distance};

// Whether a string matches a query, and how well
let (matched, score) = fuzzy_match("apple", "app");

// The distance between two strings
let distance = levenshtein_distance("apple", "apples");
```

### Vector and hybrid search

```rust
use foodshare_search::{calculate_hybrid_score, cosine_similarity, apply_rrf};

let similarity = cosine_similarity(&query, &item);
let score = calculate_hybrid_score(&query, &item, &weights);
let ranked = apply_rrf(&result_lists, 60);
```

## WASM Usage

```typescript
import init, { search_items } from '@foodshare/search-wasm';

await init();

const items = JSON.stringify(['apple', 'banana', 'apricot']);
const results = JSON.parse(search_items('app', items, 10));
```

## Performance

| Items | Query Time |
|-------|------------|
| 1,000 | ~0.5ms |
| 10,000 | ~5ms |
| 100,000 | ~50ms |

## Links

- [crates.io](https://crates.io/crates/foodshare-search)
- [npm](https://www.npmjs.com/package/@foodshare/search-wasm)
