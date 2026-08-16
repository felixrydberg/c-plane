#!/usr/bin/env node
// Common S3 API compatibility and latency benchmark (Node.js standard library only).

import crypto from 'node:crypto'

if (process.argv.includes('--help') || process.argv.includes('-h')) {
  console.log(`Usage: node scripts/benchmark-s3.mjs

Required: S3_BUCKET
Optional: S3_ENDPOINT (http://localhost:8081), AWS_REGION (eu-north-1),
         S3_BENCHMARK_ITERATIONS (10), S3_BENCHMARK_CONCURRENCY (1),
         S3_BENCHMARK_PAYLOAD_BYTES (1024), S3_BENCHMARK_TIMEOUT_MS (10000)`)
  process.exit(0)
}

const env = (name, fallback) => process.env[name] ?? fallback
const positiveInt = (name, fallback) => {
  const value = Number.parseInt(env(name, fallback), 10)
  if (!Number.isInteger(value) || value < 1) throw new Error(`${name} must be a positive integer.`)
  return value
}

const accessKey = 'CPF7ECD9A20E9A4C059CDA709498E10ABA'
const secretKey = 'edccfbe3d5fd4e1fb1b4d6ac74ceb4ae66de02646bd74077989f79377402acd9'
const bucket = 'test'
if (!bucket) throw new Error('Set S3_BUCKET.')

const endpoint = new URL(env('S3_ENDPOINT', 'http://localhost:8081'))
const region = env('AWS_REGION', 'eu-north-1')
const iterations = positiveInt('S3_BENCHMARK_ITERATIONS', 10)
const concurrency = positiveInt('S3_BENCHMARK_CONCURRENCY', 1)
const payloadBytes = positiveInt('S3_BENCHMARK_PAYLOAD_BYTES', 1024)
const timeoutMs = positiveInt('S3_BENCHMARK_TIMEOUT_MS', 10_000)
if (!['http:', 'https:'].includes(endpoint.protocol)) throw new Error('S3_ENDPOINT must be an http(s) URL.')

const encode = value => encodeURIComponent(value).replace(/[!'()*]/g, character => `%${character.charCodeAt(0).toString(16).toUpperCase()}`)
const path = (...parts) => `/${parts.map(encode).join('/')}`
const hmac = (key, value) => crypto.createHmac('sha256', key).update(value).digest()
const sha256 = value => crypto.createHash('sha256').update(value).digest('hex')
const normalizeHeader = value => String(value).trim().replace(/\s+/g, ' ')

async function request(method, requestPath, { query = {}, headers = {}, body = Buffer.alloc(0), expected }) {
  const url = new URL(endpoint)
  url.pathname = requestPath
  const queryString = Object.entries(query)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([key, value]) => `${encode(key)}=${encode(value)}`)
    .join('&')
  url.search = queryString ? `?${queryString}` : ''

  const payload = Buffer.from(body)
  const amzDate = new Date().toISOString().replace(/[:-]|\.\d{3}/g, '')
  const date = amzDate.slice(0, 8)
  const signed = {
    host: url.host,
    'x-amz-content-sha256': sha256(payload),
    'x-amz-date': amzDate,
    ...Object.fromEntries(Object.entries(headers).map(([key, value]) => [key.toLowerCase(), normalizeHeader(value)])),
  }
  const signedHeaders = Object.keys(signed).sort().join(';')
  const canonicalHeaders = Object.keys(signed).sort().map(key => `${key}:${signed[key]}\n`).join('')
  const canonicalRequest = [method, url.pathname, queryString, canonicalHeaders, signedHeaders, signed['x-amz-content-sha256']].join('\n')
  const scope = `${date}/${region}/s3/aws4_request`
  const stringToSign = `AWS4-HMAC-SHA256\n${amzDate}\n${scope}\n${sha256(canonicalRequest)}`
  const signingKey = hmac(hmac(hmac(hmac(`AWS4${secretKey}`, date), region), 's3'), 'aws4_request')
  const signature = crypto.createHmac('sha256', signingKey).update(stringToSign).digest('hex')

  const response = await fetch(url, {
    method,
    body: payload.length ? payload : undefined,
    signal: AbortSignal.timeout(timeoutMs),
    headers: {
      ...signed,
      Authorization: `AWS4-HMAC-SHA256 Credential=${accessKey}/${scope}, SignedHeaders=${signedHeaders}, Signature=${signature}`,
    },
  })
  const responseBody = Buffer.from(await response.arrayBuffer())
  if (!expected.includes(response.status)) {
    throw new Error(`HTTP ${response.status}: ${responseBody.toString('utf8').slice(0, 200)}`)
  }
  return responseBody
}

const stats = new Map()
const failures = []
async function measure(name, operation) {
  const started = performance.now()
  try {
    const result = await operation()
    const entry = stats.get(name) ?? { durations: [], failures: 0 }
    entry.durations.push(performance.now() - started)
    stats.set(name, entry)
    return result
  } catch (error) {
    const entry = stats.get(name) ?? { durations: [], failures: 0 }
    entry.durations.push(performance.now() - started)
    entry.failures += 1
    stats.set(name, entry)
    failures.push(`${name}: ${error instanceof Error ? error.message : error}`)
  }
}

const payload = Buffer.alloc(payloadBytes, 'x')
const prefix = env('S3_BENCHMARK_PREFIX', `benchmark-${Date.now()}-${process.pid}`)

async function worker(workerNumber) {
  for (let iteration = 0; iteration < iterations; iteration += 1) {
    const key = `${prefix}/${workerNumber}-${iteration}.bin`
    const copyKey = `${key}.copy`
    const objectPath = path(bucket, ...key.split('/'))
    const copyPath = path(bucket, ...copyKey.split('/'))
    try {
      await measure('list-buckets', () => request('GET', '/', { expected: [200] }))
      await measure('head-bucket', () => request('HEAD', path(bucket), { expected: [200] }))
      await measure('get-bucket-location', () => request('GET', path(bucket), { query: { location: '' }, expected: [200] }))
      await measure('list-objects-v2', () => request('GET', path(bucket), { query: { 'list-type': '2' }, expected: [200] }))
      await measure('put-object', () => request('PUT', objectPath, { body: payload, expected: [200] }))
      await measure('head-object', () => request('HEAD', objectPath, { expected: [200] }))
      await measure('get-object', async () => {
        const body = await request('GET', objectPath, { expected: [200] })
        if (!body.equals(payload)) throw new Error(`body mismatch: expected ${payload.length} bytes`)
        return body
      })
      await measure('copy-object', () => request('PUT', copyPath, {
        headers: { 'x-amz-copy-source': path(bucket, ...key.split('/')) },
        expected: [200],
      }))
      await measure('delete-copy-object', () => request('DELETE', copyPath, { expected: [204] }))
      await measure('delete-object', () => request('DELETE', objectPath, { expected: [204] }))
    } finally {
      await request('DELETE', copyPath, { expected: [204, 404] }).catch(() => {})
      await request('DELETE', objectPath, { expected: [204, 404] }).catch(() => {})
    }
  }
}

const started = performance.now()
await Promise.all(Array.from({ length: concurrency }, (_, index) => worker(index)))
const elapsed = (performance.now() - started) / 1000
const total = [...stats.values()].reduce((sum, entry) => sum + entry.durations.length, 0)

console.log(`S3 compatibility benchmark: ${endpoint.origin}`)
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
