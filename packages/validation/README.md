# @foodshare/validation

Shared Zod validation schemas for the FoodShare platform.

## Installation

```bash
bun add @foodshare/validation
```

## Usage

```typescript
import { userSchema, createListingSchema } from '@foodshare/validation'

// Validate user data
const user = userSchema.parse(data)

// Validate listing creation
const listing = createListingSchema.parse(input)
```

## Available Schemas

- **User**: `userSchema`, `createUserSchema`, `updateUserSchema`
- **Listing**: `listingSchema`, `createListingSchema`, `updateListingSchema`
- **Review**: `reviewSchema`, `createReviewSchema`
- **Common**: `emailSchema`, `uuidSchema`, `paginationSchema`, etc.

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
