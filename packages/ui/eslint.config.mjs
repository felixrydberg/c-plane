// @ts-check
import withNuxt from './.nuxt/eslint.config.mjs'

export default withNuxt(
  {
    rules: {
      'no-restricted-syntax': [
        'error',
        {
          selector:
            "CallExpression[callee.property.name='where'] > LogicalExpression[operator='&&']",
          message: 'Use drizzle and()/or() inside .where(), never &&/||',
        },
      ],
    },
  },
)
