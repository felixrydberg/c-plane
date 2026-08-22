export type Organization = {
  id: string;
  name: string;
  slug: string;
  created_at: string;
  logo: string | null;
  member: {
    id: string;
    role: string;
    permissions: string[];
  };
}
