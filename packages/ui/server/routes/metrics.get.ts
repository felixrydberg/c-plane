import { metricsText } from "../utils/metrics"

export default defineEventHandler(async (event) => {
  setHeader(event, "Content-Type", "text/plain; version=0.0.4; charset=utf-8")
  return metricsText()
})
