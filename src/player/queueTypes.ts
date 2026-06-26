export type QueueItem =
  | { kind: 'local'; id: string; title: string }   // id=path, title=basename
  | { kind: 'remote'; id: string; title: string }  // id=canonical watchUrl；playAt 時才 resolveRemote

// 單片 remote 貼上後、標題尚未抓回前的暫顯哨兵。QueuePanel 依此顯示 spinner；
// 回填/playAt 一旦改寫 title 即離開此值 → spinner 消失（單一真相來源）。
export const REMOTE_TITLE_LOADING = '讀取中…'
