# @foodshare/types

Shared TypeScript types for the FoodShare platform.

## Installation

```bash
bun add @foodshare/types
```

## Usage

```typescript
import { User, Listing, ApiResponse } from '@foodshare/types'

const user: User = {
  id: '123',
  email: 'user@example.com',
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString()
}
```

## Development

```bash
# Build
bun run build

# Watch mode
bun run dev

# Type check
bun run type-check
```

## License

MIT
