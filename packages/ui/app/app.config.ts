export default defineAppConfig({
  ui: {
    colors: {
      primary: 'coral',
      secondary: 'graphite',
      neutral: 'graphite',
    },
    button: {
      slots: {
        base: 'rounded-md',
      },
      variants: {
        variant: {
          frosted: {
            base: 'text-white bg-[linear-gradient(to_bottom,color-mix(in_oklch,var(--button-frosted-color),white_15%),var(--button-frosted-color))] hover:bg-[linear-gradient(to_bottom,color-mix(in_oklch,var(--button-frosted-color),white_22%),color-mix(in_oklch,var(--button-frosted-color),black_4%))] ring-1 ring-inset ring-[color-mix(in_oklch,var(--button-frosted-color),black_10%)] shadow-[inset_0_1px_0_color-mix(in_oklch,var(--button-frosted-color),white_45%),inset_0_-1px_0_color-mix(in_oklch,var(--button-frosted-color),black_18%),0_2px_4px_rgba(33,33,33,0.18)] focus-visible:outline-3 disabled:opacity-75 aria-disabled:opacity-75',
          },
        },
      },
      compoundVariants: [
        { color: 'primary', variant: 'frosted', class: '[--button-frosted-color:var(--ui-primary)] focus-visible:outline-primary/25' },
        { color: 'secondary', variant: 'frosted', class: '[--button-frosted-color:var(--cp-surface-subtle)] text-default shadow-none focus-visible:outline-inverted/25 dark:[--button-frosted-color:var(--ui-color-neutral-900)] dark:text-white' },
        { color: 'success', variant: 'frosted', class: '[--button-frosted-color:var(--ui-success)] focus-visible:outline-success/25' },
        { color: 'info', variant: 'frosted', class: '[--button-frosted-color:var(--ui-info)] focus-visible:outline-info/25' },
        { color: 'warning', variant: 'frosted', class: '[--button-frosted-color:var(--ui-warning)] focus-visible:outline-warning/25' },
        { color: 'error', variant: 'frosted', class: '[--button-frosted-color:var(--ui-error)] focus-visible:outline-error/25' },
        { color: 'neutral', variant: 'frosted', class: '[--button-frosted-color:var(--cp-surface-subtle)] text-default shadow-none focus-visible:outline-inverted/25 dark:[--button-frosted-color:var(--ui-color-neutral-900)] dark:text-white' },
      ],
      defaultVariants: {
        variant: 'frosted',
      },
    },
    card: {
      slots: {
        root: 'rounded-md shadow-none',
      },
    },
    formField: {
      slots: {
        label: 'font-space-mono text-[11px] font-normal uppercase tracking-[0.08em]',
      },
    },
    select: {
      defaultVariants: {
        variant: 'outline',
      }
    },
    input: {
      defaultVariants: {
        variant: 'outline',
      }
    },
    tabs: {
      defaultVariants: {
        variant: 'pill'
      }
    },
    selectMenu: {
      defaultVariants: {
        variant: 'outline',
      }
    },
    textarea: {
      defaultVariants: {
        variant: 'outline',
      }
    },
    theme: {
      colors: [
        "coral",
        "graphite"
      ]
    },
  },
})
