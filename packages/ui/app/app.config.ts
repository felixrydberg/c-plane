export default defineAppConfig({
  ui: {
    colors: {
      primary: 'coral',
      neutral: 'graphite',
    },
    button: {
      slots: {
        base: 'rounded-md',
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
