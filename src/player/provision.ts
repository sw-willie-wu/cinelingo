import type { EngineStatus, MissingAsset } from './backend'

export interface ProvisionSummary {
  missing: MissingAsset[]
  totalMb: number
  noModel: boolean
}

/** 由 check_engine 結果算出總下載量與「是否完全沒有模型」。 */
export function provisionSummary(status: EngineStatus): ProvisionSummary {
  const missing = status.missing
  const totalMb = missing.reduce((s, m) => s + m.sizeMb, 0)
  const noModel = missing.some((m) => m.kind === 'model')
  return { missing, totalMb, noModel }
}
