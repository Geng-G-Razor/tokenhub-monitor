---
version: 1.0
updated: 2026-06-26
project: tokenhub-monitor
overrides: 覆盖全局 AGENTS.md 中关于 release/发版的部分
---

# AGENTS.md — tokenhub-monitor

## 项目一句话

Tauri 2 桌面端菜单栏 app，监控 tokenhub.cash 套餐用量；Windows + macOS 双平台。
本机是 Windows。

## Release 发版（两种模式，二选一）

进入会话后第一件事：判断本任务该走 **模式 A（默认）** 还是 **模式 B（显式 CI）**。

### 模式 A — 本地构建手发（默认）

适用：用户说"打包发版 / 发 Windows / 出包"，没说"CI 出包 / 让 CI 跑"。

1. 在本地直接 `pnpm tauri build`，产物在：
   - `src-tauri/target/release/bundle/msi/*.msi`（WiX 中文 MSI）
   - `src-tauri/target/release/bundle/nsis/*-setup.exe`（NSIS 安装器）
2. 决定版本号（见下方"版本号策略"）。
3. `gh release create <tag> --title <tag> --notes "..." <msi> <nsis>`，**不打 git tag**。
4. 不要 push `v*` 形式 tag——会触发 CI 浪费 7+ 分钟。
5. 只发 Windows 两个文件，不要顺手挂 macOS dmg（本机无法验证）。

### 模式 B — CI 出包（仅当用户明确要求）

适用：用户明确说"让 CI 打包 / 跑 workflow / 我要看 CI 跑过"。

1. push 修复 commit + `v*` tag。
2. **不要等 `create-release` job**——它的 `needs: [build-macos, build-windows]`
   任一失败（哪怕只是 cleanup 步骤）都会拖死整个发布。
3. 自检：`gh api repos/<owner>/<repo>/actions/runs/<run_id>/artifacts`，
   **只看 artifact 是否上传**，不要看 run 的 overall `conclusion`。
4. artifact 在就立刻 `gh release create` / `gh release upload` 手发。

### 通用硬约束（两个模式都生效）

- **只发用户明确要求的平台**：本机无法验证的资产（macOS dmg）不主动挂。
- **不发出去的 CI 产物默认作废**：CI artifact 90 天过期，不要把"CI 跑过了"
  当成"该发版"的依据——用户没说要就不发。
- **`git commit` / `git push` 仍要用户明确指示**（继承全局 AGENTS.md 规则，
  不能因为"反正都要发版"就自动 commit）。

## 版本号策略

- **patch bump（0.1.x）**：CI/workflow 修复、文档、依赖更新；不发版就改个号也行。
- **minor bump（0.x.0）**：新功能、UI 变更。
- **major bump（x.0.0）**：破坏性变更、平台支持增删。
- **不许同日发两个 patch 版本**——如果同一天发了 0.1.5 和 0.1.6，说明中间那次判断错了，
  回退那次发版（见下）。

## 已知坑（技术细节，下次踩到直接查这里）

- **pnpm 9 + pnpm-workspace.yaml**：必须有 `packages` 字段，否则 `setup-node@v4
  (cache:pnpm)` 会卡在 `pnpm store path` 校验阶段。`allowBuilds` 是错别字段，
  正确名是 `onlyBuiltDependencies: [esbuild]`。本地 `pnpm install` 也会被这个坑卡。
- **macOS build 步骤**（仅当重新启用模式 B + macOS 时用）：
  `RUSTFLAGS: "-A unexpected_cfgs"`，绕过 `objc 0.2.7` 的 `sel_impl!` 宏生成的
  `cfg(cargo-clippy)` 警告（rustc 1.96 默认 `-D warnings`）。
- **setup-rust-lang/setup-rust-toolchain@v1**：用 `target:`（单数），不是 `targets:`。

## 发版错了怎么补救

- **release 资产挂错/挂多**：`gh release delete-asset <tag> <asset> --repo ... --yes`。
- **整个 release 不该发**：`gh release delete <tag> --repo ... --yes`。
- **本地 git commit 错了**：用 `git reset --hard HEAD~1` 回退（push 前）；push 后用
  `git revert <sha>` 新建一个反向 commit（不要 force-push，会污染别人）。
- **不该发的 git tag**：本地 `git tag -d <tag>` + `git push origin :<tag>` 删远端。