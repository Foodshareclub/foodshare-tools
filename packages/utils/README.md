# @foodshare/utils

Shared utility functions for the FoodShare platform.

## Installation

```bash
bun add @foodshare/utils
```

## Usage

```typescript
import { formatDate, getDistanceInKm, slugify } from '@foodshare/utils'

// Date formatting
const formatted = formatDate(new Date())

// Distance calculation
const distance = getDistanceInKm(
  { latitude: 40.7128, longitude: -74.0060 },
  { latitude: 34.0522, longitude: -118.2437 }
)

// String utilities
const slug = slugify('Hello World!')
```

## Available Utilities

- **Date**: `formatDate`, `formatDateTime`, `timeAgo`
- **Format**: `formatNumber`, `formatCurrency`, `truncate`
- **Geo**: `getDistanceInKm`, `formatDistance`
- **String**: `capitalize`, `slugify`, `randomString`

## Development

```bash
# Build
bun run build

# Watch mode
bun run dev

# Type check
bun run type-check

# Test
bun run test
```

## License

MIT
