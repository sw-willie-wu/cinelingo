export type MaxIconKind = 'maximize' | 'restore'

// 最大化鈕該畫哪種 SVG:已最大化 → restore(雙框),否則 → maximize(單框)。
export function maxIconKind(isMaximized: boolean): MaxIconKind {
  return isMaximized ? 'restore' : 'maximize'
}
