<script setup lang="ts">
/**
 * PlayerIcon — Cinelingo 控制列圖示集
 * 全部為手繪 inline SVG（非 icon 字型），線條風格對齊 Lucide：
 * stroke-width 1.7、round linecap/linejoin。可用 color / font-size 控制顏色與大小。
 *
 * 用法：<PlayerIcon name="play" :size="22" />
 */
export type IconName =
  | 'prev' | 'play' | 'pause' | 'next'
  | 'back5' | 'forward5'
  | 'volume' | 'mute'
  | 'captions-live' | 'tracks' | 'settings' | 'playlist' | 'pip'
  | 'fullscreen' | 'fullscreen-exit'
  | 'pin'
  | 'close' | 'back' | 'grid' | 'minimize' | 'maximize' | 'restore'
  | 'floating-subs'
  | 'adjust'
  | 'more'
  | 'mic'
  | 'speed' | 'quality' | 'image' | 'normalize' | 'audiodelay'
  | 'home'

withDefaults(defineProps<{ name: IconName; size?: number }>(), { size: 20 })
</script>

<template>
  <!-- prev -->
  <svg v-if="name === 'prev'" :width="size" :height="size" viewBox="0 0 24 24" fill="currentColor">
    <rect x="5.5" y="5.5" width="2.4" height="13" rx="1.1" />
    <path d="M19 6.3v11.4a1 1 0 0 1-1.54.84l-8.7-5.7a1 1 0 0 1 0-1.68l8.7-5.7A1 1 0 0 1 19 6.3Z" />
  </svg>

  <!-- play -->
  <svg v-else-if="name === 'play'" :width="size" :height="size" viewBox="0 0 24 24" fill="currentColor">
    <path d="M8 5.14v13.72a1 1 0 0 0 1.52.85l11.14-6.86a1 1 0 0 0 0-1.7L9.52 4.29A1 1 0 0 0 8 5.14Z" />
  </svg>

  <!-- pause -->
  <svg v-else-if="name === 'pause'" :width="size" :height="size" viewBox="0 0 24 24" fill="currentColor">
    <rect x="6.5" y="5" width="3.6" height="14" rx="1.3" />
    <rect x="13.9" y="5" width="3.6" height="14" rx="1.3" />
  </svg>

  <!-- next -->
  <svg v-else-if="name === 'next'" :width="size" :height="size" viewBox="0 0 24 24" fill="currentColor">
    <rect x="16.1" y="5.5" width="2.4" height="13" rx="1.1" />
    <path d="M5 6.3v11.4a1 1 0 0 0 1.54.84l8.7-5.7a1 1 0 0 0 0-1.68L6.54 5.46A1 1 0 0 0 5 6.3Z" />
  </svg>

  <!-- back5 (後退:雙三角快退 ⏪) -->
  <svg v-else-if="name === 'back5'" :width="size" :height="size" viewBox="0 0 24 24" fill="currentColor">
    <path d="M20.5 5.5 12 12 20.5 18.5z" />
    <path d="M12 5.5 3.5 12 12 18.5z" />
  </svg>

  <!-- forward5 (前進:雙三角快進 ⏩) -->
  <svg v-else-if="name === 'forward5'" :width="size" :height="size" viewBox="0 0 24 24" fill="currentColor">
    <path d="M3.5 5.5 12 12 3.5 18.5z" />
    <path d="M12 5.5 20.5 12 12 18.5z" />
  </svg>

  <!-- volume -->
  <svg v-else-if="name === 'volume'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <path d="M11 5L6.5 9H3.5a1 1 0 0 0-1 1v4a1 1 0 0 0 1 1h3L11 19a.6.6 0 0 0 1-.45V5.45A.6.6 0 0 0 11 5Z" fill="currentColor" stroke="none" />
    <path d="M15.5 9.2a4 4 0 0 1 0 5.6" />
    <path d="M18.2 6.6a7.5 7.5 0 0 1 0 10.8" />
  </svg>

  <!-- mute -->
  <svg v-else-if="name === 'mute'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <path d="M11 5L6.5 9H3.5a1 1 0 0 0-1 1v4a1 1 0 0 0 1 1h3L11 19a.6.6 0 0 0 1-.45V5.45A.6.6 0 0 0 11 5Z" fill="currentColor" stroke="none" />
    <path d="M16.5 10l4 4M20.5 10l-4 4" />
  </svg>

  <!-- captions-live (字幕 / YT 風格：圓角矩形 + 文字線) -->
  <svg v-else-if="name === 'captions-live'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="5" width="18" height="14" rx="3.5" />
    <path d="M7 10.5h4M14 10.5h3M7 14h3M13 14h4" />
  </svg>

  <!-- tracks (字幕音軌 / captions) -->
  <svg v-else-if="name === 'tracks'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="5" width="18" height="14" rx="3.5" />
    <path d="M10 10.4a2.3 2.3 0 1 0 0 3.2" />
    <path d="M16.2 10.4a2.3 2.3 0 1 0 0 3.2" />
  </svg>

  <!-- settings (gear / cog) -->
  <svg v-else-if="name === 'settings'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
    <circle cx="12" cy="12" r="3" />
  </svg>

  <!-- playlist (list-video) -->
  <svg v-else-if="name === 'playlist'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <path d="M4 7h12M4 12h12M4 17h8" />
    <path d="M18 13.2l4 2.3-4 2.3z" fill="currentColor" />
  </svg>

  <!-- pip (picture-in-picture) -->
  <svg v-else-if="name === 'pip'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round">
    <rect x="3" y="5" width="18" height="14" rx="2.5" />
    <rect x="11.5" y="11" width="7.5" height="5.5" rx="1.4" fill="currentColor" stroke="none" />
  </svg>

  <!-- fullscreen -->
  <svg v-else-if="name === 'fullscreen'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <path d="M4 9V5.5A1.5 1.5 0 0 1 5.5 4H9" />
    <path d="M15 4h3.5A1.5 1.5 0 0 1 20 5.5V9" />
    <path d="M20 15v3.5a1.5 1.5 0 0 1-1.5 1.5H15" />
    <path d="M9 20H5.5A1.5 1.5 0 0 1 4 18.5V15" />
  </svg>

  <!-- fullscreen-exit (退出全螢幕：四角往內凹) -->
  <svg v-else-if="name === 'fullscreen-exit'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <path d="M9 4v3.5A1.5 1.5 0 0 1 7.5 9H4" />
    <path d="M15 4v3.5A1.5 1.5 0 0 0 16.5 9H20" />
    <path d="M20 15h-3.5a1.5 1.5 0 0 0-1.5 1.5V20" />
    <path d="M4 15h3.5A1.5 1.5 0 0 1 9 16.5V20" />
  </svg>

  <!-- pin (釘選最上層 / pushpin) -->
  <svg v-else-if="name === 'pin'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <path d="M12 17v5" />
    <path d="M9 10.8a2 2 0 0 1-1.1 1.8l-1.8.9A2 2 0 0 0 5 15.2V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.8a2 2 0 0 0-1.1-1.8l-1.8-.9A2 2 0 0 1 15 10.8V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z" />
  </svg>

  <!-- close -->
  <svg v-else-if="name === 'close'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round">
    <path d="M6 6l12 12M18 6L6 18" />
  </svg>

  <!-- back -->
  <svg v-else-if="name === 'back'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
    <path d="M14 6l-6 6 6 6" />
  </svg>

  <!-- grid (媒體櫃) -->
  <svg v-else-if="name === 'grid'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <rect x="4" y="5" width="7" height="6" rx="1.5" />
    <rect x="13" y="5" width="7" height="6" rx="1.5" />
    <rect x="4" y="13" width="7" height="6" rx="1.5" />
    <rect x="13" y="13" width="7" height="6" rx="1.5" />
  </svg>

  <!-- minimize -->
  <svg v-else-if="name === 'minimize'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
    <path d="M5 12h14" />
  </svg>

  <!-- maximize -->
  <svg v-else-if="name === 'maximize'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9">
    <rect x="5" y="5" width="14" height="14" rx="2.5" />
  </svg>

  <!-- restore (還原：雙框) -->
  <svg v-else-if="name === 'restore'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <rect x="8" y="8" width="11" height="11" rx="2.2" />
    <path d="M5.5 15V7a1.5 1.5 0 0 1 1.5-1.5h8" />
  </svg>

  <!-- floating-subs：外移視窗框 + 字幕線（浮動字幕條） -->
  <svg v-else-if="name === 'floating-subs'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="5" width="18" height="11" rx="2" />
    <line x1="6.5" y1="9.5" x2="14" y2="9.5" />
    <line x1="6.5" y1="12.5" x2="11" y2="12.5" />
    <path d="M7 19h10" />
  </svg>

  <!-- adjust（三橫線滑桿：tune / sliders） -->
  <svg v-else-if="name === 'adjust'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <line x1="4" y1="7" x2="20" y2="7" />
    <circle cx="8" cy="7" r="2.2" fill="currentColor" stroke="none" />
    <line x1="4" y1="12" x2="20" y2="12" />
    <circle cx="15" cy="12" r="2.2" fill="currentColor" stroke="none" />
    <line x1="4" y1="17" x2="20" y2="17" />
    <circle cx="10" cy="17" r="2.2" fill="currentColor" stroke="none" />
  </svg>

  <!-- more（水平三點：更多 / 設定） -->
  <svg v-else-if="name === 'more'" :width="size" :height="size" viewBox="0 0 24 24" fill="currentColor">
    <circle cx="5" cy="12" r="1.9" />
    <circle cx="12" cy="12" r="1.9" />
    <circle cx="19" cy="12" r="1.9" />
  </svg>

  <!-- mic（麥克風） -->
  <svg v-else-if="name === 'mic'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <rect x="9" y="2" width="6" height="11" rx="3"/>
    <path d="M19 10a7 7 0 0 1-14 0"/>
    <line x1="12" y1="19" x2="12" y2="23"/>
    <line x1="8" y1="23" x2="16" y2="23"/>
  </svg>

  <!-- speed（速度錶：播放速度） -->
  <svg v-else-if="name === 'speed'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <path d="M4.5 18a8 8 0 1 1 15 0" />
    <path d="M12 13.5 15.5 10" />
    <circle cx="12" cy="13.5" r="1.1" fill="currentColor" stroke="none" />
  </svg>

  <!-- quality（顯示器：畫質） -->
  <svg v-else-if="name === 'quality'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="4.5" width="18" height="12" rx="2" />
    <line x1="9" y1="20" x2="15" y2="20" />
    <line x1="12" y1="16.5" x2="12" y2="20" />
  </svg>

  <!-- image（對比半圓：影像調整） -->
  <svg v-else-if="name === 'image'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="9" />
    <path d="M12 3a9 9 0 0 0 0 18Z" fill="currentColor" stroke="none" />
  </svg>

  <!-- normalize（音量條：音量正規化） -->
  <svg v-else-if="name === 'normalize'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <line x1="5" y1="9" x2="5" y2="15" />
    <line x1="10" y1="4.5" x2="10" y2="19.5" />
    <line x1="15" y1="7" x2="15" y2="17" />
    <line x1="20" y1="10.5" x2="20" y2="13.5" />
  </svg>

  <!-- home（房子：關閉影片回首頁） -->
  <svg v-else-if="name === 'home'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <path d="M2.5 11 12 3.5 21.5 11" />
    <path d="M4.5 10V20h15V10" />
    <path d="M9.5 20v-5.5h5V20" />
  </svg>

  <!-- audiodelay（時鐘：音訊延遲） -->
  <svg v-else-if="name === 'audiodelay'" :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="9" />
    <path d="M12 7v5l3.5 2" />
  </svg>
</template>
