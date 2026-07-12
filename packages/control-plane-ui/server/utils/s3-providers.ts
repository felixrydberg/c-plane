type SerializedProviderInput = {
  id: string;
  provider_type: "aws_s3" | "cloudflare_r2";
  endpoint_url: string;
  provider_region: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
};

export const serializeProvider = (provider: SerializedProviderInput) => ({
  id: provider.id,
  provider_type: provider.provider_type,
  endpoint_url: provider.endpoint_url,
  provider_region: provider.provider_region,
  is_active: provider.is_active,
  created_at: provider.created_at,
  updated_at: provider.updated_at,
});
