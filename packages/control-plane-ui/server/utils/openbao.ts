import { createError } from "h3";

type S3ProviderCredentials = {
  access_key_id: string;
  secret_access_key: string;
  session_token?: string;
};

const getConfig = () => {
  const address = process.env.NUXT_OPENBAO_ADDR;
  const token = process.env.NUXT_OPENBAO_TOKEN;

  if (!address || !token) {
    throw createError({
      statusCode: 500,
      statusMessage: "Missing OpenBao configuration",
    });
  }

  return { address: address.replace(/\/$/, ""), token };
};

const providerPath = (providerId: string) => `platform/s3/providers/${providerId}`;

const request = async (path: string, init: RequestInit) => {
  const { address, token } = getConfig();
  const response = await fetch(`${address}/v1/cplane/data/${path}`, {
    ...init,
    headers: {
      "X-Vault-Token": token,
      "Content-Type": "application/json",
      ...init.headers,
    },
  });

  if (!response.ok) {
    throw createError({
      statusCode: 502,
      statusMessage: "OpenBao request failed",
    });
  }
};

export const writeS3ProviderCredentials = (providerId: string, credentials: S3ProviderCredentials) =>
  request(providerPath(providerId), {
    method: "POST",
    body: JSON.stringify({ data: credentials }),
  });

export const deleteS3ProviderCredentials = (providerId: string) =>
  request(providerPath(providerId), { method: "DELETE" });
