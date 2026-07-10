// ponytail: shared between signin/signup, moved here from duplicate inline definitions
export const getQueryValue = (value: unknown): string | undefined => {
  if (Array.isArray(value)) {
    return typeof value[0] === 'string' ? value[0] : undefined
  }
  return typeof value === 'string' ? value : undefined
}

export const useAuthSwitchQuery = () => {
  const route = useRoute()
  return computed(() => {
    const params = new URLSearchParams()
    const email = getQueryValue(route.query.email)
    const redirect = getQueryValue(route.query.redirect)
    const redirectTo = getQueryValue(route.query.redirectTo)
    if (email) params.set('email', email)
    if (redirect) params.set('redirect', redirect)
    if (redirectTo) params.set('redirectTo', redirectTo)
    const query = params.toString()
    return query.length > 0 ? `?${query}` : ''
  })
}
