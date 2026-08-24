import { active_organization, organization, organization_member } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { uuidv7 } from "uuidv7";
import z from "zod";
import { activeOrganizationScope, getIdentityDb, withTenantDb } from "~~/server/utils/db";
import { logEvent } from "~~/server/utils/events";

const createOrganizationSchema = z.object({
  name: z.string().trim().min(1, "Name is required"),
  email: z.string().trim().email("Invalid email address").transform((value) => value.toLowerCase()),
  slug: z
    .string()
    .trim()
    .min(1, "Slug is required")
    .transform((value) => value.toLowerCase())
    .refine((value) => ORGANIZATION_SLUG_REGEX.test(value), "Invalid organization slug"),
});

export default defineEventHandler(async (event) => {
  const session = await requireSession(event);
  const body = await readBody(event);
  const parsed = createOrganizationSchema.safeParse(body);

  if (!parsed.success) {
    throw createError({
      statusCode: 400,
      statusMessage: parsed.error.issues[0]?.message || "Invalid request body",
    });
  }

  const { name, email, slug } = parsed.data;

  const existingSlug = await getIdentityDb()
    .select({ id: organization.id })
    .from(organization)
    .where(eq(organization.slug, slug))
    .limit(1);
  if (existingSlug.length > 0) {
    throw createError({
      statusCode: 409,
      statusMessage: "Organization slug is already in use",
    });
  }

  const existingEmail = await getIdentityDb()
    .select({ id: organization.id })
    .from(organization)
    .where(eq(organization.email, email))
    .limit(1);
  if (existingEmail.length > 0) {
    throw createError({
      statusCode: 409,
      statusMessage: "Organization email is already in use",
    });
  }

  const organizationId = uuidv7();
  const organizationMemberId = uuidv7();

  const organizationScope = await activeOrganizationScope(session.user.id, organizationId);
  const createdOrganization = await (async () => {
    try {
      return await withTenantDb(organizationScope, async (tx) => {
        const [created] = await tx.insert(organization).values({
          id: organizationId,
          name,
          email,
          slug,
        }).returning({
          id: organization.id,
          name: organization.name,
          slug: organization.slug,
          created_at: organization.created_at,
          logo: organization.logo,
        });

        await tx.insert(organization_member).values({
          id: organizationMemberId,
          organization_id: organizationId,
          user_id: session.user.id,
          role: "owner",
        });

        await logEvent(
          organizationId,
          "organization:created",
          {
            summary: `Created organization '${name}'`,
            target_id: organizationId,
          },
          false,
          { actor_id: session.user.id },
          tx,
        );

        await tx
          .insert(active_organization)
          .values({
            user_id: session.user.id,
            organization_id: organizationId,
          })
          .onConflictDoUpdate({
            target: active_organization.user_id,
            set: {
              organization_id: organizationId,
            },
          });

        return created;
      });
    } catch (error) {
      if (typeof error === "object" && error && "code" in error && error.code === "23505") {
        throw createError({
          statusCode: 409,
          statusMessage: "Organization slug or email is already in use",
        });
      }
      throw error;
    }
  })();

  if (!createdOrganization) {
    throw createError({ statusCode: 500, statusMessage: "Failed to create organization" });
  }

  return {
    ...createdOrganization,
    member: {
      id: session.user.id,
      role: "owner",
    },
  };
});
