<script setup lang="ts">
import type { ContextMenuItem, TableColumn, TableRow } from "@nuxt/ui";
import { h } from "vue";

const limit = 20;
const offset = ref(0);
const NuxtTime = resolveComponent("NuxtTime");

const { data, status, refresh } = await useFetch<{
  data: Array<{
    verification_id: number;
    verification_public_id: string;
    organization_id: string;
    organization_name: string;
    verification_status: string;
    manual_review_status: string;
    manual_review_reason: string[];
    review_notes: string | null;
    reviewed_by: string | null;
    reviewed_at: string | null;
    created_at: string;
    verdict: string | null;
  }>;
  pagination: {
    total: number;
    limit: number;
    offset: number;
  };
}>("/api/admin/manual-reviews", {
  query: {
    limit,
    offset,
  },
});

type ManualReview = NonNullable<typeof data.value>["data"][number];

const columns: TableColumn<ManualReview>[] = [
  {
    accessorKey: "verification_public_id",
    header: "Verification",
  },
  {
    accessorKey: "organization_name",
    header: "Organization",
  },
  {
    accessorKey: "manual_review_status",
    header: "Review Status",
  },
  {
    accessorKey: "verdict",
    header: "Verdict",
  },
  {
    accessorKey: "created_at",
    header: "Created At",
    cell: (item) => h("div", [
      h(NuxtTime, {
        datetime: item.row.original.created_at,
        locale: "en",
        dateStyle: "short",
        timeStyle: "short",
        hour12: false,
      }),
    ]),
  },
];

const items = computed(() => data.value?.data ?? [] as ManualReview[]);
const total = computed(() => data.value?.pagination.total ?? 0);

const reviewModalOpen = ref(false);
const selectedVerificationId = ref<number | undefined>();
const selectedVerificationPublicId = ref<string | undefined>();

const openReviewModal = (row: TableRow<ManualReview>) => {
  selectedVerificationId.value = row.original.verification_id;
  selectedVerificationPublicId.value = row.original.verification_public_id;
  reviewModalOpen.value = true;
};

const getContextMenuItems = (row: TableRow<ManualReview>): ContextMenuItem[] => {
  return [
    {
      type: "label" as const,
      label: "Actions",
    },
    {
      label: "Open Review",
      onSelect: () => openReviewModal(row),
    },
  ];
};

const onReviewed = async () => {
  await refresh();
};
</script>

<template>
  <UDashboardPanel id="admin_manual_reviews">
    <template #body>
      <UiPageContainer title="Manual Reviews">
        <UiTable
          v-model:offset="offset"
          :columns="columns"
          :items="items"
          :status="status ?? 'pending'"
          :get-context-menu-items="getContextMenuItems"
          :pagination="true"
          :total="total"
          :limit="limit"
        />

        <AdminManualReviewModal
          v-model="reviewModalOpen"
          :verification-id="selectedVerificationId"
          :verification-public-id="selectedVerificationPublicId"
          @reviewed="onReviewed"
        />
      </UiPageContainer>
    </template>
  </UDashboardPanel>
</template>
