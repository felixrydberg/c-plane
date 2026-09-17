import {
  Counter,
  Histogram,
  Registry,
} from "@prometheus-io/client"

export const metricsRegistry = new Registry()

export const httpRequests = new Counter({
  name: "cplane_ui_http_requests_total",
  help: "Total HTTP requests handled by the UI",
  labelNames: ["method", "route_group", "status"] as const,
  registers: [metricsRegistry],
})

export const httpRequestDuration = new Histogram({
  name: "cplane_ui_http_request_duration_seconds",
  help: "HTTP request duration in seconds",
  labelNames: ["method", "route_group"] as const,
  registers: [metricsRegistry],
})

export const serverErrors = new Counter({
  name: "cplane_ui_server_errors_total",
  help: "Total UI server errors",
  labelNames: ["route_group"] as const,
  registers: [metricsRegistry],
})

export const authRequests = new Counter({
  name: "cplane_ui_auth_requests_total",
  help: "Total authentication activity",
  labelNames: ["operation", "outcome"] as const,
  registers: [metricsRegistry],
})

export const sessionChecks = new Counter({
  name: "cplane_ui_session_checks_total",
  help: "Total session checks",
  labelNames: ["outcome"] as const,
  registers: [metricsRegistry],
})

export type RouteGroup = "auth" | "ui_api" | "assets" | "page" | "other"
export type AuthOperation = "session" | "sign_in" | "sign_up" | "sign_out" | "passkey" | "other"

export function routeGroup(path: string): RouteGroup {
  if (path === "/metrics") return "other"
  if (path.startsWith("/ui-api/auth")) return "auth"
  if (path === "/ui-api" || path.startsWith("/ui-api/")) return "ui_api"
  if (path.startsWith("/_nuxt/") || path.startsWith("/favicon")) return "assets"
  if (!path.startsWith("/api/") && !path.startsWith("/_")) return "page"
  return "other"
}

export function authOperation(path: string): AuthOperation {
  if (path.includes("/get-session")) return "session"
  if (path.includes("/sign-in")) return "sign_in"
  if (path.includes("/sign-up")) return "sign_up"
  if (path.includes("/sign-out")) return "sign_out"
  if (path.includes("/passkey")) return "passkey"
  return "other"
}

export function authOutcome(status: number): "success" | "client_error" | "server_error" {
  if (status >= 500) return "server_error"
  if (status >= 400) return "client_error"
  return "success"
}

export function recordAuthActivity(path: string, status: number) {
  authRequests.inc({ operation: authOperation(path), outcome: authOutcome(status) })
}

export function recordSessionCheck(success: boolean) {
  sessionChecks.inc({ outcome: success ? "success" : "failure" })
}

export function metricsText() {
  return metricsRegistry.metrics()
}
