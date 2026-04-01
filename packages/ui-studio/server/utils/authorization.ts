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
