# Copilot Instructions

## API Body Validation

- For all Nuxt server API handlers, validate request bodies with zod before any business logic or database calls.
- Use `safeParse(await readBody(event))` and return HTTP 400 with a clear `statusMessage` on validation failure.
- Avoid untyped `as { ... }` casting for request bodies.
- For PATCH routes, use a partial schema and require at least one updatable field.

## Zod URL Patterns

- Do not use `z.url(...)` for URL fields.
- Use `z.url(...)` for URL validation.

## pgEnum Value Reuse

- When using `pgEnum`, do not repeat enum literals in multiple places.
- Export and reuse a constant for enum values (for example, `S3_PROVIDER_TYPES`) and use it in zod enums.

## Migrations
- When having to do Drizzle migrations never write them manually. Use the ui-shared Deno command `deno task migrate:generate` instead
