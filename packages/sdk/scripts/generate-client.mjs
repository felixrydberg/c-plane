import { readFile, writeFile } from 'node:fs/promises'

const methods = new Set(['get', 'post', 'put', 'patch', 'delete', 'options', 'head', 'trace'])
const spec = JSON.parse(await readFile('openapi.json', 'utf8'))
const tree = { operations: {}, children: {} }
const binaryOperations = []

const binaryResponseType = operation =>
  Object.values(operation.responses ?? {}).some(response =>
    Object.hasOwn(response.content ?? {}, 'application/octet-stream'),
  )
    ? 'blob'
    : undefined

for (const [path, pathItem] of Object.entries(spec.paths ?? {})) {
  for (const [method, operation] of Object.entries(pathItem)) {
    if (!methods.has(method)) continue
    if (!operation.operationId) throw new Error(`${method.toUpperCase()} ${path} has no operationId`)
    if (!operation.tags?.length) throw new Error(`${method.toUpperCase()} ${path} has no tag`)

    let node = tree
    const tags = operation.tags[0].split('/')
    for (const tag of tags) {
      node.children[tag] ??= { operations: {}, children: {} }
      node = node.children[tag]
    }
    const prefix = `${tags.at(-1)}_`
    const name = operation.operationId.startsWith(prefix)
      ? operation.operationId.slice(prefix.length)
      : operation.operationId
    if (node.operations[name]) {
      throw new Error(`Duplicate SDK operation: ${name}`)
    }
    const parseAs = binaryResponseType(operation)
    node.operations[name] = { method, path, operationId: operation.operationId, parseAs }
    if (parseAs) binaryOperations.push({ operationId: operation.operationId, parseAs })
  }
}

let generated = await readFile('src/generated.ts', 'utf8')
for (const operation of binaryOperations) {
  if (operation.parseAs !== 'blob') continue
  const operationName = operation.operationId.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const responseType = new RegExp(`(${operationName}:\\s*\\{[\\s\\S]*?"application/octet-stream":\\s*)(?:string|Blob)(?=;)`)
  if (!responseType.test(generated)) {
    throw new Error(`Could not update binary response type for ${operation.operationId}`)
  }
  generated = generated.replace(responseType, '$1Blob')
}
await writeFile('src/generated.ts', generated)

const quote = value => JSON.stringify(value)
const operationType = ({ method, path }) => `Operation<${quote(path)}, '${method}'>`

function renderType(node, indent = '  ') {
  const entries = []
  for (const [name, operation] of Object.entries(node.operations)) {
    entries.push(`${indent}${name}: ${operationType(operation)}`)
  }
  for (const [name, value] of Object.entries(node.children)) {
    entries.push(`${indent}${name}: {\n${renderType(value, `${indent}  `)}\n${indent}}`)
  }
  return entries.sort().join('\n')
}

function renderClient(node, indent = '    ') {
  const entries = []
  for (const [name, operation] of Object.entries(node.operations)) {
    const type = operationType(operation)
    const args = operation.parseAs
      ? `{ ...args[0], parseAs: ${quote(operation.parseAs)} }`
      : '...args'
    entries.push(`${indent}${name}: (...args: Parameters<${type}>) => client.${operation.method.toUpperCase()}(${quote(operation.path)}, ${args})`)
  }
  for (const [name, value] of Object.entries(node.children)) {
    entries.push(`${indent}${name}: {\n${renderClient(value, `${indent}  `)}\n${indent}}`)
  }
  return entries.sort().join(',\n')
}

const output = `import type { ClientForPath } from 'openapi-fetch'
import type { paths } from './generated.ts'
import { createRawClient, type SdkOptions } from './runtime.ts'

type Operation<Path extends keyof paths, Method extends keyof paths[Path] & string> =
  ClientForPath<paths[Path], 'application/json'>[
    Uppercase<Method> & keyof ClientForPath<paths[Path], 'application/json'>
  ]

export type Sdk = {
${renderType(tree)}
}

export const createSdk = (options: SdkOptions = {}): Sdk => {
  const client = createRawClient(options)
  return {
${renderClient(tree)}
  } as Sdk
}
`

await writeFile('src/client.ts', output)
