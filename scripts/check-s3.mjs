#!/usr/bin/env node
// Minimal authenticated S3 ListBuckets probe (Node.js standard library only).

import crypto from 'node:crypto'

const accessKey = 'CPEBA5D684AD414B358ADB9EBFC2DB5CCD'
const secretKey = '5ea0464b09084dc3afb8aaf0b894d7f5c8afb1a8e7d24802952ab15443b015a2'
const endpoint = new URL(process.env.S3_ENDPOINT ?? 'http://localhost:8081')
const region = process.env.AWS_REGION ?? 'local'

if (!['http:', 'https:'].includes(endpoint.protocol)) throw new Error('S3_ENDPOINT must be an http(s) URL.')

const hmac = (key, value) => crypto.createHmac('sha256', key).update(value).digest()
const now = new Date()
const amzDate = now.toISOString().replace(/[:-]|\.\d{3}/g, '')
const date = amzDate.slice(0, 8)
const payloadHash = crypto.createHash('sha256').update('').digest('hex')
const signedHeaders = 'host;x-amz-content-sha256;x-amz-date'
const canonicalHeaders = `host:${endpoint.host}\nx-amz-content-sha256:${payloadHash}\nx-amz-date:${amzDate}\n`
const canonicalRequest = `GET\n/\n\n${canonicalHeaders}\n${signedHeaders}\n${payloadHash}`
const scope = `${date}/${region}/s3/aws4_request`
const stringToSign = `AWS4-HMAC-SHA256\n${amzDate}\n${scope}\n${crypto.createHash('sha256').update(canonicalRequest).digest('hex')}`
const signingKey = hmac(hmac(hmac(hmac(`AWS4${secretKey}`, date), region), 's3'), 'aws4_request')
const signature = crypto.createHmac('sha256', signingKey).update(stringToSign).digest('hex')

endpoint.pathname = '/'
endpoint.search = ''
const response = await fetch(endpoint, {
  headers: {
    Authorization: `AWS4-HMAC-SHA256 Credential=${accessKey}/${scope}, SignedHeaders=${signedHeaders}, Signature=${signature}`,
    'x-amz-content-sha256': payloadHash,
    'x-amz-date': amzDate,
  },
})
const body = await response.text()
console.log(`${response.ok ? 'S3 is working' : 'S3 rejected the request'}: HTTP ${response.status}`)
console.log(body)
if (!response.ok) process.exitCode = 1
