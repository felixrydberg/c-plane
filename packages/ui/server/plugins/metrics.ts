import { defineNitroPlugin } from "nitropack/runtime"
import { getRequestURL } from "h3"
import { httpRequestDuration, httpRequests, recordAuthActivity, routeGroup, serverErrors } from "../utils/metrics"

export default defineNitroPlugin((nitroApp) => {
  nitroApp.hooks.hook("request", (event) => {
    if (getRequestURL(event).pathname !== "/metrics") {
      event.context.metricsStart = performance.now()
    }
  })

  nitroApp.hooks.hook("afterResponse", (event) => {
    const path = getRequestURL(event).pathname
    if (path === "/metrics") return
    const status = event.node.res.statusCode || 200
    const group = routeGroup(path)
    const method = event.method
    httpRequests.inc({ method, route_group: group, status: String(status) })
    const start = event.context.metricsStart
    if (typeof start === "number") httpRequestDuration.observe({ method, route_group: group }, (performance.now() - start) / 1000)
    if (group === "auth") recordAuthActivity(path, status)
  })

  nitroApp.hooks.hook("error", (_error, { event }) => {
    if (!event) return
    const path = getRequestURL(event).pathname
    if (path === "/metrics") return
    serverErrors.inc({ route_group: routeGroup(path) })
  })
})
