# Custom migrations

Keep hand-written SQL migrations in this directory. The files are applied in lexical filename order by `clean-migrations.ts`.

Run `deno task --cwd packages/migrations migrate:clean` from the repository root to regenerate the Drizzle baseline and recreate every custom migration.

Do not edit generated files under `drizzle/`; they are disposable.
