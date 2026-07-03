import type { EngineStatus, MissingAsset } from './backend'

export interface ProvisionSummary {
  missing: MissingAsset[]
  totalMb: number
}

/** 由 check_engine 結果算出總下載量。 */
export function provisionSummary(status: EngineStatus): ProvisionSummary {
  const missing = status.missing
  const totalMb = missing.reduce((s, m) => s + m.sizeMb, 0)
  return { missing, totalMb }
}
