# Button Style Guide

## Rules

1. **Frosted by default** - major action buttons use the global `frosted` variant from the Nuxt UI theme. Do not recreate it with page-level classes.
2. **Text + icon** - action buttons include both an icon and a label. Continue and Next are text-only flow-navigation exceptions, along with Cancel and Back.
3. **No X marks** - never use an X mark for Cancel, Back, Close, or Dismiss.
4. **Nuxt UI `<UButton>`** - always use the built-in component.

## Variants

Not every button should be primary. `color` communicates **severity**, not importance - match it to what happens when clicked.

### Frosted (default)

The Nuxt UI theme applies `variant="frosted"` by default. Major actions should rely on that default.

```vue
<UButton :icon="ICONS.plus">New Container</UButton>
```

### Primary

For the single most important action on a page: Create, Save, Deploy. Use sparingly - one per view.

```vue
<UButton :icon="ICONS.plus" color="primary">New Container</UButton>
<UButton :icon="ICONS.check" color="primary" :loading="saving" @click="save">Save</UButton>
```

### Neutral (secondary)

For actions without destructive or primary weight: Edit, Settings, Link, Manage, Hide, Graph. The default choice when primary is already claimed.

```vue
<UButton :icon="ICONS.general" color="neutral" @click="openSettings">Settings</UButton>
<UButton :icon="ICONS.plus" color="neutral" @click="addEnvRow">Add</UButton>
```

### Destructive

For irreversible actions: Delete, Remove.

```vue
<UButton :icon="ICONS.trash" color="error" @click="handleDelete">Delete</UButton>
```

### Cancellation / Back

For backing out of a modal or returning to the previous step, use a text-only `variant="ghost"` button. Do not use an X mark.

```vue
<UButton color="neutral" variant="ghost" @click="modalOpen = false">Cancel</UButton>
```

### Flow navigation

Continue and Next advance a flow without confirming or saving. Keep them text-only; do not add a checkmark or other status icon.

```vue
<UButton type="submit">Continue</UButton>
```

## Icon catalog

Use these icon keys from `~/utils/icons`:

| Key | Icon | Usage |
|-----|------|-------|
| `plus` | `i-heroicons:plus` | Create / Add |
| `trash` | `i-heroicons:trash` | Delete / Remove |
| `pencil` | `i-heroicons:pencil-square` | Edit |
| `check` | `i-heroicons:check` | Confirm / Save |
| `refresh` | `i-heroicons:arrow-path` | Retry / Refresh |
| `download` | `i-heroicons:arrow-down-tray` | Download |
| `more` | `i-heroicons:ellipsis-horizontal` | More actions |
| `calendar` | `i-heroicons:calendar-days` | Date range |
| `settings` | `i-heroicons:cog-6-tooth` | Settings / Manage |
| `chevronLeft` | `i-heroicons:chevron-left` | Previous / back navigation |
| `chevronRight` | `i-heroicons:chevron-right` | Next / forward navigation |
| `revision` | `i-heroicons:clock` | Draft / deployed revision control |

Add new keys to `packages/ui/app/utils/icons.ts` and register them here.

## Size

- Page-level actions: default (no `size` prop)
- Inline / table-row actions: `size="xs"` or `size="sm"`

## Loading

Every async action gets a loading state:

```vue
<UButton :icon="ICONS.trash" color="error" :loading="deleting" @click="handleDelete">Delete</UButton>
```

The `loading` prop disables the button and shows a spinner - no separate `:disabled` binding is needed.

## Do not

- Use `variant="ghost"` or `variant="soft"` on major actions
- Use an X mark for Cancel, Back, Close, or Dismiss
- Use icon-only buttons for actions
- Recreate the frosted treatment with component-level CSS classes
