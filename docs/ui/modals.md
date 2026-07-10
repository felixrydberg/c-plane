# Modal Style Guide

## Button layout

All action buttons go at the **bottom right** of the modal body. Cancel on the left, primary action on the right.

```vue
<div class="flex justify-end gap-3 pt-2">
  <UButton variant="solid" icon="i-heroicons:x-mark" color="neutral" :disabled="loading" @click="modalOpen = false">
    Cancel
  </UButton>
  <UButton type="submit" :loading="loading" :disabled="!valid">
    Primary Action
  </UButton>
</div>
```

## Structure

- Use the `#body` slot for all modal content including action buttons. Do not use `#footer` for buttons.
- Use `pt-2` (or `pt-4` for confirmation dialogs with a body paragraph) to separate buttons from content above.
- Use consistent `gap-3` between Cancel and the primary action.

## Button roles

| Position | Variant | Icon | When |
|----------|---------|------|------|
| Left | `variant="ghost" color="neutral"` | — | Always (Cancel / Dismiss) |
| Right | `color="primary"` (default) | `ICONS.check`, `ICONS.plus` | Create / Save / Send |
| Right | `color="error"` | `ICONS.trash` | Delete / Remove |

## Confirmation dialogs

For destructive confirmation dialogs (delete, remove), use `pt-4` spacing between the body text and buttons:

```vue
<template #body>
  <p class="text-sm">Are you sure you want to delete <strong>{{ name }}</strong>?</p>
  <div class="flex justify-end gap-3 pt-4">
    <UButton variant="ghost" color="neutral" @click="modalOpen = false">Cancel</UButton>
    <UButton color="error" :icon="ICONS.trash" :loading="deleting" @click="handleDelete">Delete</UButton>
  </div>
</template>
```

## Do not

- Put buttons in `#footer` — Nuxt UI modals already have a close X in the header
- Left-align action buttons
- Use inconsistent gap sizes — always `gap-3`
