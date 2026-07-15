#!/usr/bin/env node
// Minimal authenticated S3 list/upload probe (Node.js standard library only).

import crypto from 'node:crypto'

const accessKey = 'CP19469FE6F0654ED89F42FFEF7EAFE95D'
const secretKey = '064e39b8c5ff4058b655f9c3576626f909c526f8c69a442dafe303c0c7b0cd8c'
const endpoint = new URL(process.env.S3_ENDPOINT ?? 'http://localhost:8081')
const region = process.env.AWS_REGION ?? 'eu-north-1'

if (!['http:', 'https:'].includes(endpoint.protocol)) throw new Error('S3_ENDPOINT must be an http(s) URL.')

const hmac = (key, value) => crypto.createHmac('sha256', key).update(value).digest()

const signedFetch = async (method, path, body = '') => {
  const url = new URL(endpoint)
  url.pathname = path
  url.search = ''
  const payload = Buffer.from(body)
  const amzDate = new Date().toISOString().replace(/[:-]|\.\d{3}/g, '')
  const date = amzDate.slice(0, 8)
  const payloadHash = crypto.createHash('sha256').update(payload).digest('hex')
  const signedHeaders = 'host;x-amz-content-sha256;x-amz-date'
  const canonicalHeaders = `host:${url.host}\nx-amz-content-sha256:${payloadHash}\nx-amz-date:${amzDate}\n`
  const canonicalRequest = `${method}\n${url.pathname}\n\n${canonicalHeaders}\n${signedHeaders}\n${payloadHash}`
  const scope = `${date}/${region}/s3/aws4_request`
  const stringToSign = `AWS4-HMAC-SHA256\n${amzDate}\n${scope}\n${crypto.createHash('sha256').update(canonicalRequest).digest('hex')}`
  const signingKey = hmac(hmac(hmac(hmac(`AWS4${secretKey}`, date), region), 's3'), 'aws4_request')
  const signature = crypto.createHmac('sha256', signingKey).update(stringToSign).digest('hex')

  return fetch(url, {
    method,
    body: payload.length ? payload : undefined,
    headers: {
      Authorization: `AWS4-HMAC-SHA256 Credential=${accessKey}/${scope}, SignedHeaders=${signedHeaders}, Signature=${signature}`,
      'x-amz-content-sha256': payloadHash,
      'x-amz-date': amzDate,
    },
  })
}

const response = await signedFetch('GET', '/')
const body = await response.text()
console.log(`${response.ok ? 'S3 is working' : 'S3 rejected the request'}: HTTP ${response.status}`)
console.log(body)
if (!response.ok) process.exit(1)

const bucket = process.env.S3_BUCKET ?? 'test'
const key = process.env.S3_TEST_KEY ?? 'check-s3-upload.txt'
const objectPath = `/${bucket}/${key}`
const content = 'c-plane S3 upload test\n'
const upload = await signedFetch('PUT', objectPath, content)
console.log(`Upload ${upload.ok ? 'succeeded' : 'failed'}: s3://${bucket}/${key} HTTP ${upload.status}`)
const uploadResponse = await upload.text()
if (uploadResponse) console.log(uploadResponse)
if (!upload.ok) process.exit(1)

const download = await signedFetch('GET', objectPath)
const downloadedContent = await download.text()
console.log(`Fetch ${download.ok ? 'succeeded' : 'failed'}: s3://${bucket}/${key} HTTP ${download.status}`)
console.log(downloadedContent)
if (!download.ok || downloadedContent !== content) {
  console.error('Fetched content does not match the uploaded content.')
  process.exit(1)
}
