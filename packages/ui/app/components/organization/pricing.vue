<script setup lang="ts">
  const emits = defineEmits<{
    (e: 'select-plan', planId: string): void;
  }>();

  const tiers = ref([
    {
      id: 'pay-as-you-go',
      title: 'Pay As You Go',
      price: '$0',
      description: 'For indie hackers.',
      billingCycle: '/month',
      button: {
        label: 'Buy now',
        variant: 'soft' as const,
        onClick: () => emits('select-plan', 'pay-as-you-go')
      }
    },
    {
      id: 'pro',
      title: 'Pro',
      price: '$199',
      description: 'For growing teams.',
      billingCycle: '/month',
      button: {
        label: 'Buy now',
        onClick: () => emits('select-plan', 'pro')
      },
      highlight: true,
    },
    {
      id: 'enterprise',
      title: 'Enterprise',
      price: '$999',
      description: 'For large organizations.',
      billingCycle: '/month',
      button: {
        label: 'Buy now',
        onClick: () => emits('select-plan', 'enterprise')
      }
    }
  ]);

  const sections = ref([
    {
      title: 'Features',
      features: [
        {
          title: 'Included Manual Reviews',
          tiers: {
            'pay-as-you-go': 0,
            pro: '200',
            enterprise: '1 000'
          }
        },
        {
          title: 'Additional Manual Reviews',
          tiers: {
            'pay-as-you-go': '$100 / 100 reviews',
            pro: '$80 / 100 reviews',
            enterprise: '$60 / 100 reviews',
          }
        },
        {
          title: 'Included Automatic Reviews',
          tiers: {
            'pay-as-you-go': 0,
            pro: '10 000',
            enterprise: '100 000'
          }
        },
        {
          title: 'Additional Automatic Reviews',
          tiers: {
            'pay-as-you-go': '$10 / 100 reviews',
            pro: '$8 / 100 reviews',
            enterprise: '$6 / 100 reviews',
          }
        }
      ]
    },
  ]);
</script>

<template>
  <UPricingTable :tiers="tiers" :sections="sections" class="mb-3">
    <template #pro-title="{ tier }">
      <div class="flex items-center gap-2">
        <UIcon name="i-lucide-crown" class="size-4" />
        {{ tier.title }}
      </div>
    </template>

    <template #section-security-title="{ section }">
      <div class="flex items-center gap-2">
        <UIcon name="i-lucide-shield-check" class="size-4" />
        <span class="font-semibold">{{ section.title }}</span>
      </div>
    </template>

    <template #feature-e2ee-title="{ feature }">
      <span>
        {{ feature.title }}
        <sup><a href="#encryption-footnote" class="hover:underline">1</a></sup>
      </span>
    </template>
    <template #feature-desktop-client-title="{ feature }">
      <span>
        {{ feature.title }}
        <sup><a href="#desktop-client-footnote" class="hover:underline">2</a></sup>
      </span>
    </template>
  </UPricingTable>
</template>
