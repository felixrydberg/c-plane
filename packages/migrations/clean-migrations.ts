import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const packageDir = dirname(fileURLToPath(import.meta.url));
const migrationsDir = join(packageDir, "drizzle");
const customDir = join(packageDir, "custom-migrations");

async function runDrizzle(args: string[]) {
  const command = new Deno.Command(Deno.execPath(), {
    args: ["x", "--allow-read", "--allow-write", "-p", "drizzle-kit@1.0.0-rc.4", "drizzle-kit", ...args],
    cwd: packageDir,
    stdout: "inherit",
    stderr: "inherit",
  });
  const result = await command.output();
  if (!result.success) {
    throw new Error(`Drizzle command failed with exit code ${result.code}`);
  }
}

async function generatedMigrationDirs(): Promise<string[]> {
  const directories: string[] = [];
  for await (const entry of Deno.readDir(migrationsDir)) {
    if (!entry.isDirectory) continue;
    const directory = join(migrationsDir, entry.name);
    try {
      const migration = await Deno.stat(join(directory, "migration.sql"));
      if (migration.isFile) directories.push(directory);
    } catch (error) {
      if (!(error instanceof Deno.errors.NotFound)) throw error;
    }
  }
  return directories.sort();
}

async function customMigrationFiles(): Promise<string[]> {
  const files: string[] = [];
  for await (const entry of Deno.readDir(customDir)) {
    if (entry.isFile && entry.name.endsWith(".sql")) {
      files.push(join(customDir, entry.name));
    }
  }
  return files.sort();
}

function migrationTimestamp(directory: string): string {
  const match = basename(directory).match(/^(\d{14})_/);
  if (!match) {
    throw new Error(`Invalid generated migration directory: ${directory}`);
  }
  return match[1];
}

function currentMigrationTimestamp(): string {
  return new Date().toISOString().replace(/\D/g, "").slice(0, 14);
}

async function waitForNextMigrationTimestamp(initialMigrations: string[]) {
  const latestInitialTimestamp = initialMigrations
    .map(migrationTimestamp)
    .sort()
    .at(-1)!;

  while (currentMigrationTimestamp() <= latestInitialTimestamp) {
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

try {
  await Deno.stat(customDir);
  console.log(`Removing generated migrations from ${migrationsDir}`);
  await Deno.remove(migrationsDir, { recursive: true });
  await Deno.mkdir(migrationsDir, { recursive: true });

  await runDrizzle(["generate", "--config=drizzle.config.ts"]);
  const initialMigrations = await generatedMigrationDirs();
  if (initialMigrations.length === 0) {
    throw new Error("Drizzle did not generate an initial migration");
  }
  console.log("Generated initial migration(s):");
  for (const migration of initialMigrations) console.log(`  ${migration}`);

  const customFiles = await customMigrationFiles();
  if (customFiles.length > 0) {
    // Drizzle timestamps migrations to the second. Ensure custom migrations
    // sort after the baseline when they are generated in the same run.
    await waitForNextMigrationTimestamp(initialMigrations);
  }

  for (const customFile of customFiles) {
    const name = basename(customFile, ".sql");
    const before = await generatedMigrationDirs();
    console.log(`Generating custom migration: ${name}`);
    await runDrizzle(["generate", "--custom", "--config=drizzle.config.ts", `--name=${name}`]);

    const after = await generatedMigrationDirs();
    const generated = after.filter((directory) => !before.includes(directory));
    if (generated.length !== 1) {
      throw new Error(`Expected one generated migration for ${name}, found ${generated.length}`);
    }

    if (generated[0] <= initialMigrations.at(-1)!) {
      throw new Error(`Custom migration ${name} was generated before the initial migration`);
    }

    const generatedFile = join(generated[0], "migration.sql");
    console.log(`Installing ${customFile} as ${generatedFile}`);
    await Deno.copyFile(customFile, generatedFile);
  }

  console.log("Migration cleanup complete");
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  Deno.exit(1);
}
