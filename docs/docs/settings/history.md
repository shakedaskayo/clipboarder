# History limits

Two independent controls govern how much clipboarder remembers.

## Maximum items

The hard cap on **non-pinned** rows in the database.

- Defaults to **500**
- Options: 100 / 250 / 500 / 1,000 / 2,500 / 5,000 / **Unlimited**
- Enforced on every clipboard event and at startup
- When the cap is exceeded, the oldest non-pinned rows (by `last_used_at`) are deleted along with any associated image files

Choosing **Unlimited** disables the cap; the database grows until you clear it manually or hit disk limits.

## Auto-clear after

Removes non-pinned items older than N days, based on `last_used_at` (which is bumped every time you re-copy or paste an item back).

- Defaults to **Never**
- Options: 1 day / 1 week / 1 month / 3 months / 1 year
- Enforced at startup; also applied as part of capture-time housekeeping when the option is non-zero

## Pinned items are never affected

Both limits skip pinned items entirely. Pin anything you want to keep forever.

## Clear all history

The danger button at the bottom of the History section, and the **Clear history** entry in the tray menu, wipe all non-pinned rows in one shot. There's a confirmation dialog. Pinned items survive.

## Estimating storage

| Items | Approx DB size | Notes |
|-------|----------------|-------|
| 500   | ~2 MB | mostly text + FTS index |
| 5,000 | ~20 MB | excludes captured PNGs |
| Images | varies | each capture is the source PNG size (often 50–500 KB) |

The FTS5 index roughly doubles the storage of indexed text. Images are stored as-is on disk, not in the DB.
