export interface ComputeUnit {
  label: string
  cpu: number
  ramGib: number
}

export const COMPUTE_UNITS: ComputeUnit[] = [
  { label: '0.25', cpu: 0.25, ramGib: 0.5 },
  { label: '0.5', cpu: 0.5, ramGib: 1 },
  { label: '1', cpu: 1, ramGib: 2 },
  { label: '2', cpu: 2, ramGib: 4 },
  { label: '4', cpu: 4, ramGib: 8 },
  { label: '8', cpu: 8, ramGib: 16 },
  { label: '16', cpu: 16, ramGib: 32 },
  { label: '32', cpu: 32, ramGib: 64 },
]

export const COMPUTE_UNIT_ITEMS = COMPUTE_UNITS.map(u => ({ label: u.label, value: u.label }))

export function findComputeUnit(cpu: number, ramGib: number): ComputeUnit | undefined {
  return COMPUTE_UNITS.find(u => u.cpu === cpu && u.ramGib === ramGib)
}

export function computeUnitByLabel(label: string): ComputeUnit | undefined {
  return COMPUTE_UNITS.find(u => u.label === label)
}

/**
 * Convert a raw CPU string (e.g. "0.5") and RAM in MiB (e.g. "2048Mi") to the closest compute unit label.
 */
export function resolveComputeUnitLabel(cpu: string | null, ram: string | null): string {
  if (!cpu && !ram) return '0.5'
  const c = cpu ? Number.parseFloat(cpu) || 0.5 : 0.5
  const rGib = ram ? parseFloatRamGib(ram) : 1
  const match = findComputeUnit(c, rGib)
  return match?.label ?? `${c}c / ${rGib}G`
}

function parseFloatRamGib(ram: string): number {
  const m = ram.match(/^(\d+(?:\.\d+)?)\s*Mi$/i)
  return m ? Math.round(Number.parseFloat(m[1]) / 1024 * 100) / 100 : Number.parseFloat(ram) || 1
}
