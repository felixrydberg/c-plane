export function pagination(query: Record<string, unknown>) {
  const parse = (value: unknown, fallback: number) => {
    const parsed = Number.parseInt(typeof value === "string" ? value : "", 10);
    return Number.isFinite(parsed) ? parsed : fallback;
  };

  return {
    limit: Math.min(Math.max(parse(query.limit, 50), 0), 100),
    offset: Math.max(parse(query.offset, 0), 0),
  };
}
