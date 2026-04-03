import { auth } from "./auth";
import type { H3Event } from "h3";

export const requireSession = async (event: H3Event) => {
  const session = await auth.api.getSession({
    headers: event.headers,
  });

  if (!session) {
    throw createError({
      statusCode: 401,
      statusMessage: "Unauthorized",
    });
  }
  return session;
};

export const requireAdmin = async (event: H3Event) => {
  const session = await requireSession(event);
  const role = (session.user as { role?: string } | undefined)?.role;

  if (!role || role.toLowerCase() !== "admin") {
    throw createError({
      statusCode: 403,
      statusMessage: "Forbidden",
    });
  }

  return session;
};
