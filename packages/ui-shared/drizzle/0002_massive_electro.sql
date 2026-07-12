ALTER TABLE "infrastructure_audit_log" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "infrastructure_audit_log" FORCE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE POLICY "infrastructure_audit_log_reader" ON "infrastructure_audit_log" AS PERMISSIVE FOR SELECT TO "app_audit_reader" USING (true);--> statement-breakpoint
REVOKE ALL PRIVILEGES ON "infrastructure_audit_log" FROM PUBLIC, ui, app_identity, app_tenant, app_admin, cplane_identity, cplane_tenant, cplane_admin;--> statement-breakpoint
GRANT INSERT ON "infrastructure_audit_log" TO app_admin;--> statement-breakpoint
GRANT USAGE ON SCHEMA public TO app_audit_reader;--> statement-breakpoint
GRANT SELECT ON "infrastructure_audit_log" TO app_audit_reader;--> statement-breakpoint
GRANT app_audit_reader TO cplane_admin WITH INHERIT FALSE;
