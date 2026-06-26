// 接受任意 http(s) URL：貼上後一律交給 yt-dlp 嘗試解析（非 YouTube 站 yt-dlp 也常支援），
// 解析不出來再回報「不支援」。非 URL（純文字）→ false → 不攔、放行原生貼上。
export function isHttpUrl(text: string): boolean {
  try {
    const u = new URL(text.trim())
    return u.protocol === 'http:' || u.protocol === 'https:'
  } catch {
    return false
  }
}

const YT_HOSTS = new Set(['youtube.com', 'www.youtube.com', 'm.youtube.com', 'music.youtube.com'])

// YouTube URL → canonical `https://www.youtube.com/watch?v=<id>`。
// 非 YouTube host / 認不出 id / 解析失敗 → 回 trim 後原樣。
// host 用 exact set membership（非 substring）→ 免 evil-youtube.com 誤中。
export function canonicalRemoteId(url: string): string {
  const trimmed = url.trim()
  let u: URL
  try {
    u = new URL(trimmed)
  } catch {
    return trimmed
  }
  const host = u.hostname.toLowerCase()
  let id: string | null = null
  if (host === 'youtu.be') {
    id = u.pathname.split('/').filter(Boolean)[0] ?? null
  } else if (YT_HOSTS.has(host)) {
    const v = u.searchParams.get('v')
    if (v) {
      id = v
    } else {
      const segs = u.pathname.split('/').filter(Boolean)
      if (segs.length >= 2 && ['live', 'embed', 'shorts', 'v'].includes(segs[0])) {
        id = segs[1]
      }
    }
  }
  return id ? `https://www.youtube.com/watch?v=${id}` : trimmed
}

// YT 清單脈絡：有 list= 參數（host 為 YT exact-set）→ {listId, videoId}；否則 null（沿用單片流程）。
// host 用 exact set membership（同 canonicalRemoteId）→ 免 evil-youtube.com 誤中。
export function playlistContext(url: string): { listId: string; videoId: string | null } | null {
  let u: URL
  try {
    u = new URL(url.trim())
  } catch {
    return null
  }
  if (!YT_HOSTS.has(u.hostname.toLowerCase())) return null
  const listId = u.searchParams.get('list')
  if (!listId) return null
  return { listId, videoId: u.searchParams.get('v') }
}
