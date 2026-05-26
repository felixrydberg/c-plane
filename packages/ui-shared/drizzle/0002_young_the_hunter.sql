ALTER TABLE "stateful_postgres_database" ADD COLUMN "cpu" text;--> statement-breakpoint
ALTER TABLE "stateful_postgres_database" ADD COLUMN "ram" text;--> statement-breakpoint
ALTER TABLE "stateful_postgres_database" ADD COLUMN "high_availability" boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE "stateful_postgres_database" ADD COLUMN "read_replicas" integer;