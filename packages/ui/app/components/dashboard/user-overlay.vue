<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import useStore from '~/stores/store';

defineProps<{
  collapsed?: boolean
}>();
const store = useStore();
const router = useRouter();
const colorMode = useColorMode();
const isDark = computed(() => colorMode.value === 'dark');

function toggleColorMode() {
  colorMode.preference = isDark.value ? 'light' : 'dark';
}
const user = computed(() => {
  return {
    name: store.user?.name,
    avatar: {
      alt: store.user?.name,
    }
  }
})

const items = computed<DropdownMenuItem[][]>(() => ([[{
  type: 'label',
  label: user.value.name,
  avatar: user.value.avatar
}], [{
  label: 'Settings',
  icon: 'i-heroicons:cog-6-tooth',
  to: '/settings'
}, {
  label: isDark.value ? 'Switch to light mode' : 'Switch to dark mode',
  icon: isDark.value ? 'i-heroicons:sun' : 'i-heroicons:moon',
  onSelect: toggleColorMode,
}, {
  label: 'Log out',
  icon: 'i-heroicons:arrow-right-start-on-rectangle',
  onSelect: () => {
    signOut();
  }
}]]))
</script>

<template>
  <UDropdownMenu
    :items="items"
    :content="{ align: 'center', collisionPadding: 12 }"
    :ui="{ content: collapsed ? 'w-48' : 'w-(--reka-dropdown-menu-trigger-width)' }"
  >
    <UButton
      v-bind="{
        ...user,
        label: collapsed ? undefined : user?.name,
        trailingIcon: collapsed ? undefined : 'i-lucide-chevrons-up-down'
      }"
      color="neutral"
      variant="soft"
      block
      :square="collapsed"
      class="data-[state=open]:bg-elevated"
      :ui="{
        trailingIcon: 'text-dimmed'
      }"
    />

    <template #chip-leading="{ item }">
      <span
        :style="{
          '--chip-light': `var(--color-${(item as any).chip}-500)`,
          '--chip-dark': `var(--color-${(item as any).chip}-400)`
        }"
        class="ms-0.5 size-2 rounded-full bg-(--chip-light) dark:bg-(--chip-dark)"
      />
    </template>
  </UDropdownMenu>
</template>
