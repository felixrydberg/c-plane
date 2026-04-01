export type Organization = {
  id: string;
  name: string;
  slug: string;
  created_at: string;
  logo: string | null;
  member: {
    id: string;
    role: string;
  };
  subscription?: {
    id: string;
    organization_id: string;
    polar_subscription_id: string;
    created_at: string;
    updated_at: string;
    status: "active" | "canceled" | "unpaid";
  } | null;
}
