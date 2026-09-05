# Custom migrations

Keep hand-written SQL migrations in this directory. The files are applied in lexical filename order by `clean-migrations.ts`.

Run `deno task --cwd packages/migrations migrate:clean` from the repository root to regenerate the Drizzle baseline and recreate every custom migration.

Do not hand-edit SQL under `drizzle/`; it is disposable. For an incremental schema
change needing custom SQL, run `migrate:generate` to capture the schema snapshot,
then copy the canonical SQL from this directory into that generated `migration.sql`.
Custom SQL must also work after `migrate:clean` creates a baseline with the new schema.
