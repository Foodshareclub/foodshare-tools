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
use foodshare_search::{FuzzySearch, SearchResult};

let items = vec!["apple", "banana", "apricot", "avocado"];
let searcher = FuzzySearch::new(&items);

let results = searcher.search("app", 10);
// Returns: ["apple", "apricot"] with scores
```

### With Custom Items

```rust
use foodshare_search::{FuzzySearch, Searchable};

struct Product {
    id: u32,
    name: String,
}

impl Searchable for Product {
    fn search_text(&self) -> &str {
        &self.name
    }
}

let products = vec![
    Product { id: 1, name: "Fresh Apples".into() },
    Product { id: 2, name: "Banana Bread".into() },
];

let searcher = FuzzySearch::new(&products);
let results = searcher.search("apple", 5);
```

## WASM Usage

```typescript
import init, { fuzzy_search } from '@foodshare/search-wasm';

await init();

const items = JSON.stringify(['apple', 'banana', 'apricot']);
const results = JSON.parse(fuzzy_search('app', items, 10));
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
