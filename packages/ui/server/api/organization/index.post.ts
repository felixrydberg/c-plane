import { active_organization, organization, organization_member } from "~~/server/schema";
import { eq } from "drizzle-orm";
import { uuidv7 } from "uuidv7";
import z from "zod";
import { getIdentityDb, withTenantDb } from "~~/server/utils/db";

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

  const existingSlug = await getIdentityDb().query.organization.findFirst({
    where: eq(organization.slug, slug),
  });
  if (existingSlug) {
    throw createError({
      statusCode: 409,
      statusMessage: "Organization slug is already in use",
    });
  }

  const existingEmail = await getIdentityDb().query.organization.findFirst({
    where: eq(organization.email, email),
  });
  if (existingEmail) {
    throw createError({
      statusCode: 409,
      statusMessage: "Organization email is already in use",
    });
  }

  const organizationId = uuidv7();
  const organizationMemberId = uuidv7();

  const createdOrganization = await withTenantDb([organizationId], async (tx) => {
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

  return {
    ...createdOrganization,
    member: {
      id: session.user.id,
      role: "owner",
    },
  };
});
