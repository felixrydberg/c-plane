import { readFile, writeFile } from 'node:fs/promises'
import openapiTS, { astToString } from 'npm:openapi-typescript'

const ast = await openapiTS(JSON.parse(await readFile('openapi.json', 'utf8')))
await writeFile('src/generated.ts', astToString(ast, { fileName: 'src/generated.ts' }))
