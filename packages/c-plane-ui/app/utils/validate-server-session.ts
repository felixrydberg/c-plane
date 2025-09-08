import type { FrontendApi } from "@ory/client";

export default async (ory: FrontendApi): Promise<ReturnType<FrontendApi["toSession"]> | null> => {
  if (import.meta.client) {
    throw createError("validateServerSession can only be called on the server");
  }
  const cookie = useCookie("ory_kratos_session");
  if (cookie.value === null) {
    return null;
  }

  return await ory.toSession({
    cookie: `ory_kratos_session=${cookie.value}`,
  });
};
