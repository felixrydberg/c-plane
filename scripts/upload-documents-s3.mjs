#!/usr/bin/env node
// Upload many document-shaped objects to an S3-compatible bucket.

import crypto from 'node:crypto'

if (process.argv.includes('--help') || process.argv.includes('-h')) {
  console.log(`Usage: node scripts/upload-documents-s3.mjs

Required: S3_BUCKET
Optional: S3_ENDPOINT (http://localhost:8081), AWS_REGION (eu-north-1),
         S3_DOCUMENT_COUNT (100), S3_DOCUMENT_CONCURRENCY (8),
         S3_DOCUMENT_SIZE_BYTES (4096), S3_DOCUMENT_PREFIX (documents-<timestamp>),
         S3_DOCUMENT_TIMEOUT_MS (10000), S3_ACCESS_KEY_ID, S3_SECRET_ACCESS_KEY`)
  process.exit(0)
}

const env = (name, fallback) => process.env[name] ?? fallback
const positiveInt = (name, fallback) => {
  const value = Number.parseInt(env(name, fallback), 10)
  if (!Number.isInteger(value) || value < 1) throw new Error(`${name} must be a positive integer.`)
  return value
}

const accessKey = env('S3_ACCESS_KEY_ID', 'CPF7ECD9A20E9A4C059CDA709498E10ABA')
const secretKey = env('S3_SECRET_ACCESS_KEY', 'edccfbe3d5fd4e1fb1b4d6ac74ceb4ae66de02646bd74077989f79377402acd9')
const bucket = env('S3_BUCKET', 'test')
if (!bucket) throw new Error('Set S3_BUCKET.')

const endpoint = new URL(env('S3_ENDPOINT', 'http://localhost:8081'))
const region = env('AWS_REGION', 'eu-north-1')
const count = positiveInt('S3_DOCUMENT_COUNT', 100)
const concurrency = positiveInt('S3_DOCUMENT_CONCURRENCY', 8)
const documentSize = positiveInt('S3_DOCUMENT_SIZE_BYTES', 4096)
const timeoutMs = positiveInt('S3_DOCUMENT_TIMEOUT_MS', 10_000)
const prefix = env('S3_DOCUMENT_PREFIX', `documents-${Date.now()}-${process.pid}`)

if (!['http:', 'https:'].includes(endpoint.protocol)) throw new Error('S3_ENDPOINT must be an http(s) URL.')

const encode = value => encodeURIComponent(value).replace(/[!'()*]/g, character => `%${character.charCodeAt(0).toString(16).toUpperCase()}`)
const path = (...parts) => `/${parts.map(encode).join('/')}`
const hmac = (key, value) => crypto.createHmac('sha256', key).update(value).digest()
const sha256 = value => crypto.createHash('sha256').update(value).digest('hex')
const normalizeHeader = value => String(value).trim().replace(/\s+/g, ' ')

async function request(method, requestPath, { headers = {}, body = Buffer.alloc(0), expected }) {
  const url = new URL(endpoint)
  url.pathname = requestPath

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
  const canonicalRequest = [method, url.pathname, '', canonicalHeaders, signedHeaders, signed['x-amz-content-sha256']].join('\n')
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
}

function document(index) {
  const header = `C-Plane benchmark document\nIndex: ${index}\nGenerated: ${new Date().toISOString()}\n\n`
  const body = `This document exists to exercise S3 object listing and rendering.\n${'Lorem ipsum dolor sit amet, consectetur adipiscing elit. '.repeat(20)}\n`
  const content = Buffer.from(`${header}${body}`)
  if (content.length >= documentSize) return content.subarray(0, documentSize)

  return Buffer.concat([content, Buffer.alloc(documentSize - content.length, ' ')])
}

const failures = []
let nextIndex = 0
let completed = 0
const progressInterval = Math.max(1, Math.floor(count / 20))

async function worker() {
  while (nextIndex < count) {
    const index = nextIndex++
    const key = `${prefix}/document-${String(index).padStart(String(count).length, '0')}.txt`

    try {
      await request('PUT', path(bucket, ...key.split('/')), {
        headers: { 'content-type': 'text/plain; charset=utf-8' },
        body: document(index),
        expected: [200, 201, 204],
      })
    } catch (error) {
      failures.push(`${key}: ${error instanceof Error ? error.message : error}`)
    } finally {
      completed += 1
      if (completed === count || completed % progressInterval === 0) {
        console.log(`Uploaded ${completed}/${count} documents`)
      }
    }
  }
}

const started = performance.now()
await Promise.all(Array.from({ length: Math.min(concurrency, count) }, () => worker()))
const elapsed = (performance.now() - started) / 1000

console.log(`S3 document upload: ${endpoint.origin}`)
console.log(`${count - failures.length}/${count} uploaded in ${elapsed.toFixed(2)}s (${(count / elapsed).toFixed(1)} objects/s)`)
console.log(`Prefix: s3://${bucket}/${prefix}/`)

if (failures.length) {
  console.error(`\n${failures.length} uploads failed:`)
  for (const failure of failures.slice(0, 10)) console.error(`- ${failure}`)
  process.exitCode = 1
}
