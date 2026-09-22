# Custom migrations

Use Drizzle Kit for ordinary schema migrations. Define tables, columns, indexes, constraints, enums, and Drizzle-supported RLS policies in `packages/migrations/schema`, then generate migrations with `deno task --cwd packages/migrations migrate:generate`.

Reserve hand-written SQL in this directory for PostgreSQL functions and triggers that Drizzle Kit cannot represent, including their updates and removals. The files are applied in lexical filename order by `clean-migrations.ts`.

Run `deno task --cwd packages/migrations migrate:clean` from the repository root to regenerate the Drizzle baseline and recreate every custom migration.

Maintainers must not author custom SQL directly under `drizzle/`; it is disposable.
When a schema change also needs a function or trigger, generate the schema migration
with Drizzle Kit and add only that function or trigger SQL from this directory.
Custom SQL must also work after `migrate:clean` creates a baseline with the new
schema.
