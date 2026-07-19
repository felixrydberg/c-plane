ALTER TABLE "postgres_database_branch" ADD COLUMN "cpu" text;--> statement-breakpoint
ALTER TABLE "postgres_database_branch" ADD COLUMN "ram" text;--> statement-breakpoint
ALTER TABLE "postgres_database_branch" ADD COLUMN "high_availability" boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE "postgres_database_branch" ADD COLUMN "read_replicas" integer;--> statement-breakpoint
ALTER TABLE "postgres_database_branch" ADD COLUMN "autoscaling_enabled" boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE "postgres_database_branch" ADD COLUMN "autoscaling_min_cpu" text;--> statement-breakpoint
ALTER TABLE "postgres_database_branch" ADD COLUMN "autoscaling_max_cpu" text;--> statement-breakpoint
ALTER TABLE "postgres_database" DROP COLUMN "cpu";--> statement-breakpoint
ALTER TABLE "postgres_database" DROP COLUMN "ram";--> statement-breakpoint
ALTER TABLE "postgres_database" DROP COLUMN "high_availability";--> statement-breakpoint
ALTER TABLE "postgres_database" DROP COLUMN "read_replicas";--> statement-breakpoint
ALTER TABLE "postgres_database" DROP COLUMN "autoscaling_enabled";--> statement-breakpoint
ALTER TABLE "postgres_database" DROP COLUMN "autoscaling_min_cpu";--> statement-breakpoint
ALTER TABLE "postgres_database" DROP COLUMN "autoscaling_max_cpu";