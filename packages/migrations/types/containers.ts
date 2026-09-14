import type { InferSelectModel } from "drizzle-orm";
import type { container } from "../schema/projects/containers.ts";

export type ContainerRow = InferSelectModel<typeof container>;
