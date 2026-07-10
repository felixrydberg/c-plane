# Button Style Guide

## Rules

1. **Solid colors only** — no ghost, outline, or soft variants. Every button has visual weight.
2. **Text + icon** — every button includes both an icon and a label. No icon-only or text-only buttons.
3. **Nuxt UI `<UButton>`** — always use the built-in component.

## Variants

Not every button should be primary. `color` communicates **severity**, not importance — match it to what happens when clicked.

### Primary

For the single most important action on a page: Create, Save, Deploy. Use sparingly — one per view.

```vue
<UButton :icon="ICONS.plus" color="primary">New Container</UButton>
<UButton :icon="ICONS.check" color="primary" :loading="saving" @click="save">Save</UButton>
```

### Neutral (secondary)

For actions without destructive or primary weight: Edit, Settings, Link, Manage, Hide, Graph. The default choice when primary is already claimed.

```vue
<UButton :icon="ICONS.general" variant="solid" color="neutral" @click="openSettings">Settings</UButton>
<UButton :icon="ICONS.plus" variant="solid" color="neutral" @click="addEnvRow">Add</UButton>
<UButton :icon="ICONS.xMark" variant="solid" color="neutral" @click="deselectRevision">Hide</UButton>
```

### Destructive

For irreversible actions: Delete, Remove.

```vue
<UButton :icon="ICONS.trash" color="error" @click="handleDelete">Delete</UButton>
```

### Dismissal / Cancel

For backing out of a modal or cancelling an in-flight operation. Use `variant="ghost"` to keep visual weight on the primary action.

```vue
<UButton color="neutral" variant="ghost" @click="modalOpen = false">Cancel</UButton>
```

## Icon catalog

Use these icon keys from `~/utils/icons`:

| Key | Icon | Usage |
|-----|------|-------|
| `plus` | `i-heroicons:plus` | Create / Add |
| `trash` | `i-heroicons:trash` | Delete / Remove |
| `pencil` | `i-heroicons:pencil-square` | Edit |
| `xMark` | `i-heroicons:x-mark` | Cancel / Close |
| `check` | `i-heroicons:check` | Confirm / Save |

Add new keys to `packages/ui/app/utils/icons.ts` and register them here.

## Size

- Page-level actions: default (no `size` prop)
- Inline / table-row actions: `size="xs"` or `size="sm"`

## Loading

Every async action gets a loading state:

```vue
<UButton :icon="ICONS.trash" color="error" :loading="deleting" @click="handleDelete">Delete</UButton>
```

The `loading` prop disables the button and shows a spinner — no need for a separate `:disabled` binding.

## Icon-only (exception)

Permitted for **chrome actions** — dismissive or navigational controls adjacent to content where text would be redundant: closing a panel, toggling sidebar, dismissing a toast. Always paired with `aria-label`. Use `variant="ghost"` to keep visual weight low.

```vue
<UButton variant="ghost" size="xs" color="neutral" :icon="ICONS.xMark" aria-label="Hide sidebar" @click="closePanel" />
```

## Do not

- `variant="ghost"` or `variant="soft"` on major actions
- Icon-only buttons without `aria-label`
- Text-only buttons that hide the affordance
- `color="neutral"` with no variant — invisible on the page
