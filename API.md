# Cinelingo — Pinned versions & plugin API reference

## Pinned versions (P1 baseline, 2026-06-14)

| 套件 | 版本 |
|---|---|
| create-tauri-app | 4.6.2（template `vue-ts`, `--tauri-version 2`） |
| @tauri-apps/api | 2.11.0 |
| @tauri-apps/cli | 2.11.2 |
| @tauri-apps/plugin-opener | 2.5.4 |
| vue | 3.5.38 |
| vite | 6.4.3 |
| vue-tsc | 2.2.12 |
| typescript | 5.6.3 |
| identifier | com.lightmpv.player |

Cargo（宣告於 `src-tauri/Cargo.toml`）：`tauri = "2"`、`tauri-build = "2"`、`tauri-plugin-opener = "2"`、`serde`/`serde_json = "1"`。確切解析版本見 `src-tauri/Cargo.lock`（首次編譯後）。

## libmpv build
- 由 `npx tauri-plugin-libmpv-api setup-lib` 取得（zhongfly mpv-winbuild）。
- libmpv 來源檔：`mpv-dev-lgpl-x86_64-20260613-git-7d245fd100.7z` → **確認為 LGPL build** ✓（檔名含 `lgpl`，非預設 GPL build）。
- `libmpv-2.dll` sha256：`aedc003730729bf7f8c8d646b3417dc20a48dbb7af7cb812c54b9267170814b9`（97 MB）。
- `libmpv-wrapper.dll`：來源 `libmpv-wrapper-windows-x86_64.zip`（plugin 提供的 wrapper，執行期載入）。
- DLL 置於 `src-tauri/lib/`，**已 gitignore（`/lib/`），不進 git**；重現用 `setup-lib` 重新下載。
- P2 散布前須履行 LGPL 義務（附授權條文、可重新連結/替換該函式庫）。

## tauri-plugin-libmpv API（自 `node_modules/tauri-plugin-libmpv-api/dist-js/*.d.ts` 確認）

**Formats**
```ts
type MpvFormat = 'string' | 'flag' | 'int64' | 'double' | 'node'
MpvFormatToType = { string: string; flag: boolean; int64: number; double: number; node: unknown }
```

**Observed properties（tuple）**
```ts
type MpvObservableProperty =
  | readonly [string, MpvFormat]
  | readonly [string, MpvFormat, 'none', ...unknown[]]   // 第三元 'none' = 值可能為 null（如未載入檔案時）
```

**Config**
```ts
interface MpvConfig {
  initialOptions?: Record<string, string | boolean | number>
  observedProperties?: readonly MpvObservableProperty[]
}
```

**Functions**
```ts
init(mpvConfig?: MpvConfig, windowLabel?: string): Promise<string>        // 回傳實際使用的 window label
destroy(windowLabel?: string): Promise<void>
observeProperties(props, callback, windowLabel?): Promise<UnlistenFn>     // callback: ({ name, data }) => void
command(name: string, args?: (string|boolean|number)[], windowLabel?): Promise<void>
setProperty(name, value: string|boolean|number, windowLabel?): Promise<void>
getProperty(name, format: MpvFormat, windowLabel?): Promise<type | null>  // 依 format 回對應型別或 null
```

**observe callback 事件形狀**：`MpvPropertyChangeEvent = { event:'property-change'; name; data; id }` → 用 `e.name` / `e.data`。

**本專案用到的屬性 → format**
| mpv 屬性 | format | 備註 |
|---|---|---|
| `pause` | `flag` | boolean |
| `path` | `string` | 完整路徑（nullable → tuple 帶 `'none'`） |
| `time-pos` | `double` | 秒（nullable） |
| `hwdec-current` | `string` | T6 驗硬解；軟解時回 `no`/空 |

> 結論:T6 的 `mpv.ts` 用 `['pause','flag'] / ['path','string','none'] / ['time-pos','double','none']`、callback `({name,data})`、`getProperty('hwdec-current','string')`，**無需任何 `as any`**。`command('cycle',['pause'])` 切換暫停;或 `setProperty('pause', bool)`。
