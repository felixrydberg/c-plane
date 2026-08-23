#!/usr/bin/env bun
// Common S3 API compatibility and latency benchmark.

import { S3Client } from 'bun'

if (process.argv.includes('--help') || process.argv.includes('-h')) {
  console.log('Usage: bun scripts/benchmark-s3.mjs')
  console.log('Uses Bun.S3Client for PUT, HEAD/existence, GET, and DELETE.')
  process.exit(0)
}

const accessKey = 'CP35CE39A8F0324463976BD1DE9F49664F'
const secretKey = '4b9245019f8045deb982b20aef3615302a0adfb4970c44dbb2be71b993d34d25'
const bucket = 'test'

const endpoint = new URL('https://storage.674571401.xyz')
const region = 'eu-north-1'
const iterations = 1
const concurrency = 1
const payloadBytes = 1024
const timeoutMs = 10_000
const debugHeadObject = true

const client = new S3Client({
  accessKeyId: accessKey,
  secretAccessKey: secretKey,
  bucket,
  endpoint: endpoint.origin,
  region,
})

async function request(operation) {
  let timer
  try {
    return await Promise.race([
      operation(),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`timeout after ${timeoutMs}ms`)), timeoutMs)
      }),
    ])
  } finally {
    clearTimeout(timer)
  }
}

const stats = new Map()
const failures = []
async function measure(name, operation) {
  const started = performance.now()
  try {
    await operation()
    const entry = stats.get(name) ?? { durations: [], failures: 0 }
    entry.durations.push(performance.now() - started)
    stats.set(name, entry)
    return true
  } catch (error) {
    const entry = stats.get(name) ?? { durations: [], failures: 0 }
    entry.durations.push(performance.now() - started)
    entry.failures += 1
    stats.set(name, entry)
    failures.push(`${name}: ${error instanceof Error ? error.message : error}`)
    return false
  }
}

const payload = Buffer.alloc(payloadBytes, 'x')
const prefix = `benchmark-${Date.now()}-${process.pid}`

async function worker(workerNumber) {
  for (let iteration = 0; iteration < iterations; iteration += 1) {
    const key = `${prefix}/${workerNumber}-${iteration}.bin`
    try {
      const objectCreated = await measure('put-object', () => request(() => client.write(key, payload)))
      if (objectCreated) {
        if (debugHeadObject) {
          console.error(JSON.stringify({
            client: 'Bun.S3Client',
            method: 'HEAD via Bun.S3Client.exists',
            endpoint: endpoint.origin,
            bucket,
            objectKey: key,
          }, null, 2))
        }
        await measure('head-object', async () => {
          const exists = await request(() => client.exists(key))
          if (!exists) throw new Error('object does not exist')
        })
        await measure('get-object', async () => {
          const body = Buffer.from(await request(() => client.file(key).arrayBuffer()))
          if (!body.equals(payload)) throw new Error(`body mismatch: expected ${payload.length} bytes`)
          return body
        })
        await measure('delete-object', () => request(() => client.delete(key)))
      }
    } finally {
      await request(() => client.delete(key)).catch(() => {})
    }
  }
}

const started = performance.now()
await Promise.all(Array.from({ length: concurrency }, (_, index) => worker(index)))
const elapsed = (performance.now() - started) / 1000
const total = [...stats.values()].reduce((sum, entry) => sum + entry.durations.length, 0)

console.log(`Bun S3 object benchmark: ${endpoint.origin}`)
console.log(`${total} requests, ${failures.length} failures, ${elapsed.toFixed(2)}s, ${(total / elapsed).toFixed(1)} req/s`)
console.log('operation              calls  failed  avg     p95     min     max')
for (const [name, entry] of stats) {
  const durations = entry.durations.toSorted((left, right) => left - right)
  const p95 = durations[Math.max(0, Math.ceil(durations.length * 0.95) - 1)]
  const average = durations.reduce((sum, duration) => sum + duration, 0) / durations.length
  console.log(`${name.padEnd(22)} ${String(durations.length).padStart(5)} ${String(entry.failures).padStart(7)} ${average.toFixed(1).padStart(6)}ms ${p95.toFixed(1).padStart(6)}ms ${durations[0].toFixed(1).padStart(6)}ms ${durations.at(-1).toFixed(1).padStart(6)}ms`)
}
if (failures.length) {
  console.error('\nFailures:')
  for (const failure of failures.slice(0, 10)) console.error(`- ${failure}`)
  process.exitCode = 1
}
