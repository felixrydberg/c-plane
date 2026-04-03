#!/bin/bash

rm -rf drizzle/
npm run migrate:generate

initial_migration=$(ls drizzle/0000_*.sql 2>/dev/null | head -1)
echo "Generated initial migration: $initial_migration"

# Array of custom migrations with their migration numbers
declare -A migrations
migrations["seed-base-data"]="0001"

# Iterate migrations in numeric order, since associative array key order is not stable.
ordered_migrations=()
while IFS= read -r line; do
  ordered_migrations+=("$line")
done < <(
  for key in "${!migrations[@]}"; do
    echo "${migrations[$key]}:$key"
  done | sort
)

for entry in "${ordered_migrations[@]}"; do
  IFS=: read -r migration_num name <<< "$entry"
  echo "Generating custom migration: $name"

  npx drizzle-kit generate --custom --name="$name"

  if [ -f "custom-migrations/$name.sql" ]; then
    generated_file=$(ls drizzle/${migration_num}_*.sql 2>/dev/null | head -1)

    if [ -f "$generated_file" ]; then
      echo "Replacing $generated_file with custom-migrations/$name.sql"
      cat "./custom-migrations/$name.sql" > "$generated_file"
    else
      echo "Warning: Could not find migration file for $name with prefix ${migration_num}"
    fi
  else
    echo "Warning: custom-migrations/$name.sql not found"
  fi
done

echo "Migration setup complete!"
