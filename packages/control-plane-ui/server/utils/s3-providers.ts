type SerializedProviderInput = {
  id: string;
  provider_type: "aws_s3" | "cloudflare_r2";
  endpoint_url: string;
  provider_region: string | null;
  access_key_id: string;
  is_active: boolean;
  has_session_token: boolean;
  has_secret_access_key: boolean;
  created_at: string;
  updated_at: string;
};

export const serializeProvider = (provider: SerializedProviderInput) => ({
  id: provider.id,
  provider_type: provider.provider_type,
  endpoint_url: provider.endpoint_url,
  provider_region: provider.provider_region,
  access_key_id: provider.access_key_id,
  is_active: provider.is_active,
  has_session_token: provider.has_session_token,
  has_secret_access_key: provider.has_secret_access_key,
  created_at: provider.created_at,
  updated_at: provider.updated_at,
});
