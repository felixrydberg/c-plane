DROP TRIGGER IF EXISTS postgres_database_branch_enqueue_backup_delete
ON public.postgres_database_branch;
DROP FUNCTION IF EXISTS public.enqueue_postgres_branch_backup_delete();
