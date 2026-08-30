import {
  bucket,
  bucket_grant,
  credential,
  managed_registry,
  organization,
  secret,
} from "~~/server/schema";
import { eq } from "drizzle-orm";
import { withTenantDb } from "~~/server/utils/db";
import { requireOwner } from "~~/server/utils/authorization";

export default defineEventHandler(async (event) => {
  const membership = await requireOwner(event);

  const organizationId = membership.organization_id;
  if (!organizationId) {
    throw createError({
      statusCode: 400,
      statusMessage: "Organization ID is required",
    });
  }

  await withTenantDb([organizationId], async (tx) => {
    const rows = await tx
      .select()
      .from(organization)
      .where(eq(organization.id, organizationId))
      .limit(1);

    if (!rows || rows.length === 0) {
      throw createError({
        statusCode: 404,
        statusMessage: "Organization not found",
      });
    }

    const [registry] = await tx
      .select()
      .from(managed_registry)
      .where(eq(managed_registry.organization_id, organizationId))
      .limit(1);
    if (registry) {
      const [registryCredential] = await tx
        .select()
        .from(credential)
        .where(eq(credential.id, registry.credential_id))
        .limit(1);
      const [foundationBucket] = await tx
        .select()
        .from(bucket)
        .where(eq(bucket.id, registry.bucket_id))
        .limit(1);

      await tx.delete(managed_registry)
        .where(eq(managed_registry.organization_id, organizationId));
      await tx.delete(bucket_grant)
        .where(eq(bucket_grant.credential_id, registry.credential_id));
      await tx.delete(credential)
        .where(eq(credential.id, registry.credential_id));
      await tx.delete(bucket).where(eq(bucket.id, registry.bucket_id));

      if (registryCredential) {
        await tx.delete(secret).where(eq(secret.id, registryCredential.secret_id));
      }
      if (foundationBucket) {
        await tx.delete(secret).where(eq(secret.id, foundationBucket.sse_secret_id));
      }
    }

    await tx.delete(organization).where(eq(organization.id, organizationId));
  });

  return { success: true };
});
