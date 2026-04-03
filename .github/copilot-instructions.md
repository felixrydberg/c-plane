# Copilot Instructions

## API Body Validation

- For all Nuxt server API handlers, validate request bodies with zod before any business logic or database calls.
- Use `safeParse(await readBody(event))` and return HTTP 400 with a clear `statusMessage` on validation failure.
- Avoid untyped `as { ... }` casting for request bodies.
- For PATCH routes, use a partial schema and require at least one updatable field.
