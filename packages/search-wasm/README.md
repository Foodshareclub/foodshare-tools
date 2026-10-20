# @foodshare/search-wasm

High-performance fuzzy search and text matching compiled to WebAssembly from Rust.

[![npm version](https://img.shields.io/npm/v/@foodshare/search-wasm.svg)](https://www.npmjs.com/package/@foodshare/search-wasm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Relevance Scoring** - Multi-level scoring (exact, starts-with, word-boundary, contains, fuzzy)
- **Fuzzy Matching** - Find matches even with typos or partial input
- **Levenshtein Distance** - Calculate edit distance between strings
- **Batch Search** - Search multiple items and return sorted results
- **TypeScript Support** - Full type definitions included

## Installation

```bash
bun add @foodshare/search-wasm
```

## Usage

### Initialization

```typescript
import init, { relevance_score, search_items } from '@foodshare/search-wasm';

// Initialize WASM module (required once)
await init();
```

### Relevance Scoring

```typescript
import init, { relevance_score } from '@foodshare/search-wasm';

await init();

// Score ranges from 0-50:
// 50 = Exact match
// 40 = Starts with
// 30 = Word boundary
// 20 = Contains
// 10 = Fuzzy match
// 0  = No match

relevance_score('hello', 'Hello');        // 50 (exact, case-insensitive)
relevance_score('hel', 'Hello World');    // 40 (starts with)
relevance_score('world', 'Hello World');  // 30 (word boundary)
relevance_score('ello', 'Hello');         // 20 (contains)
relevance_score('hwo', 'Hello World');    // 10 (fuzzy - chars in order)
relevance_score('xyz', 'Hello');          // 0  (no match)
```

### Batch Search

```typescript
import init, { search_items } from '@foodshare/search-wasm';

await init();

const items = JSON.stringify([
  { id: '1', text: 'Fresh Apples' },
  { id: '2', text: 'Apple Pie' },
  { id: '3', text: 'Banana Bread' },
  { id: '4', text: 'Pineapple' },
]);

// Search and get top 3 results
const results = JSON.parse(search_items('apple', items, 3));
// [
//   { id: '2', score: 30 },  // Word boundary match
//   { id: '1', score: 30 },  // Word boundary match
//   { id: '4', score: 20 },  // Contains
// ]
```

### Fuzzy Matching

```typescript
import init, { fuzzy_contains } from '@foodshare/search-wasm';

await init();

// Check if query characters appear in text in order
fuzzy_contains('hwo', 'Hello World');  // true (h...w...o)
fuzzy_contains('hel', 'Hello');        // true
fuzzy_contains('leh', 'Hello');        // false (wrong order)
```

### Edit Distance

```typescript
import init, { edit_distance } from '@foodshare/search-wasm';

await init();

// Number of single-character edits to transform one string into another
edit_distance('hello', 'hallo');  // 1 (substitute e->a)
edit_distance('hello', 'helo');   // 1 (delete l)
edit_distance('cat', 'dog');      // 3 (all different)
```

## API Reference

### `relevance_score(query, text): number`
Calculate relevance score (0-50) for matching query against text.

### `search_items(query, items_json, max_results): string`
Search items and return JSON array of results sorted by score.

### `fuzzy_contains(query, text): boolean`
Check if all query characters appear in text in order.

### `edit_distance(a, b): number`
Calculate Levenshtein edit distance between two strings.

## Item JSON Format

For `search_items`, provide a JSON array with `id` and `text` fields:

```typescript
interface Item {
  id: string;
  text: string;
}
```

## Scoring Levels

| Score | Match Type | Example (query: "hello") |
|-------|------------|--------------------------|
| 50 | Exact | "Hello" |
| 40 | Starts With | "Hello World" |
| 30 | Word Boundary | "Say Hello" |
| 20 | Contains | "SayHelloWorld" |
| 10 | Fuzzy | "H...e...l...l...o" |
| 0 | No Match | "Goodbye" |

## Performance

- ~10x faster than JavaScript string matching
- Processes thousands of items in milliseconds
- Zero-copy string handling with WASM

## Framework Integration

### React/Next.js

```typescript
import { useEffect, useState } from 'react';

export function useSearch() {
  const [search, setSearch] = useState<typeof import('@foodshare/search-wasm') | null>(null);

  useEffect(() => {
    import('@foodshare/search-wasm').then(async (module) => {
      await module.default();
      setSearch(module);
    });
  }, []);

  return search;
}
```

## Browser Support

Works in all modern browsers with WebAssembly support:
- Chrome 57+
- Firefox 52+
- Safari 11+
- Edge 16+

## License

MIT License - see [LICENSE](https://github.com/Foodshareclub/foodshare-tools/blob/main/LICENSE) for details.

## Related

- [foodshare-search](https://crates.io/crates/foodshare-search) - The Rust crate this package is built from
