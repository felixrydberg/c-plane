const RESERVED_REGION_SLUGS = new Set(["storage", "deployment", "global"]);

// Keep region slugs URL-safe and compatible with existing slug conventions.
export const REGION_SLUG_REGEX = /^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/;

export const normalizeRegionSlug = (value: string) => value.trim().toLowerCase();

export const isReservedRegionSlug = (value: string) => RESERVED_REGION_SLUGS.has(normalizeRegionSlug(value));
