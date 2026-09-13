CREATE OR REPLACE FUNCTION public.enqueue_foundation_bucket_delete()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    provider_id uuid;
BEGIN
    SELECT regions.s3_provider_id
    INTO provider_id
    FROM public.regions AS regions
    WHERE regions.id = OLD.region_id;

    IF provider_id IS NULL THEN
        RAISE EXCEPTION 'Cannot enqueue provider deletion for bucket %: region % has no S3 provider',
            OLD.id,
            OLD.region_id;
    END IF;

    INSERT INTO public.worker_queue (
        id,
        queue_name,
        job_type,
        dedupe_key,
        payload
    )
    VALUES (
        pg_catalog.gen_random_uuid(),
        'foundation',
        'foundation_bucket_delete',
        OLD.id::text,
        pg_catalog.jsonb_build_object(
            'bucket_id', OLD.id,
            'provider_id', provider_id
        )
    );

    RETURN OLD;
END;
$$;
--> statement-breakpoint
CREATE TRIGGER bucket_enqueue_foundation_delete
AFTER DELETE ON public.bucket
FOR EACH ROW
EXECUTE FUNCTION public.enqueue_foundation_bucket_delete();
