export default defineAppConfig({
  ui: {
    colors: {
      neutral: 'neutral',
    },
    select: {
      defaultVariants: {
        variant: 'soft',
      }
    },
    input: {
      defaultVariants: {
        variant: 'soft',
      }
    },
    selectMenu: {
      defaultVariants: {
        variant: 'soft',
      }
    },
    textarea: {
      defaultVariants: {
        variant: 'soft',
      }
    },
    buttton: {
      defaultVariants: {
        variant: 'soft',
      }
    },
    theme: {
      colors: [
        "sepia"
      ]
    },
    dashboardPanel: {
      slots: {
        body: 'flex flex-col gap-4 sm:gap-6 flex-1 overflow-y-visible p-3 sm:p-3',
      }
    },
  },
})
