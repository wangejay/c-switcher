<h1 align="center">C-Switcher</h1>

<p align="center">
  <strong>Claude Code 多帳號管理器</strong><br>
  切換 OAuth profile、監控 rate limit、自動選用量最低的帳號開工。
</p>

<p align="center">
  <img src=".github/images/image-1.png" width="600" alt="Profile 列表" />
</p>

<p align="center">
  <img src=".github/images/image-2.png" width="295" alt="用量 & Rate Limits" />
  <img src=".github/images/image-3.png" width="295" alt="新增帳號" />
</p>

<p align="center">
  <strong>▶ 操作示範影片</strong><br>
  <a href="https://www.youtube.com/watch?v=lXTgJzjNmws">
    <img src="https://img.youtube.com/vi/lXTgJzjNmws/maxresdefault.jpg" width="600" alt="操作示範影片" />
  </a>
  <br>
  <sub>點擊圖片前往 YouTube 觀看完整示範</sub>
</p>

---

## 桌面應用

原生 macOS 應用（Tauri 建置），管理 Claude Code OAuth profile。

- **Profile 列表** — 一覽所有帳號及登入狀態
- **一鍵切換** — 即時切換使用中的帳號
- **用量監控** — 即時查看 5 小時 & 7 天 rate limit
- **Token 刷新** — 免重新登入即可更新過期 token
- **引導式新增** — 步驟化流程加入新帳號

```bash
npm install && npm run tauri build
```

---

## `csw` CLI

一個指令搞定：並行查詢所有 profile 用量，選最低的，切換，啟動 `claude`。

### 安裝

```bash
cargo install --path src-tauri --bin csw
```

### 快速開始

```bash
csw                           # 自動選 5h 用量最低 → 切換 → 啟動 claude
csw -l                        # 印出用量表格後結束
csw -i                        # 互動：從表格中選 → 切換 → 啟動 claude
csw -- --resume               # 自動選 + 透傳參數給 claude
```

### 選項

| Flag | 說明 |
|---|---|
| `-l` `--list` | 印出用量表格後結束 |
| `-i` `--interactive` | 互動式選擇 profile |
| `-h` `--help` | 顯示說明 |
| `--` | 之後的參數透傳給 `claude` |

### 運作原理

```
~/.claude_profiles/*.json
        │
        ▼
   查詢用量 ──── Anthropic API（並行請求）
        │
        ▼
   依 5h% 排序 ──── 同分比 7d%
        │
        ▼
   切換 keychain + ~/.claude.json
        │
        ▼
   exec claude
```

> 表格輸出到 **stderr**，不影響 claude 的 stdout。顏色：🟢 < 50% · 🟡 50–80% · 🔴 > 80%

---

## 前置需求

| 需求 | 原因 |
|---|---|
| **macOS** | Token 存放在 Keychain |
| [Rust](https://rustup.rs/) | 建置 CLI 和 Tauri 應用 |
| Node.js + npm | 建置前端 |
| 透過桌面應用建立 Profile | `csw` 讀取 `~/.claude_profiles/` |

## 開發

```bash
npm run tauri dev
```

## License

MIT
