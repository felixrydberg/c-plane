import type { InferSelectModel } from "drizzle-orm";
import type { container, container_version } from "../schema/projects/containers.ts";

export type ContainerRow = InferSelectModel<typeof container>;
export type ContainerVersionRow = InferSelectModel<typeof container_version>;
