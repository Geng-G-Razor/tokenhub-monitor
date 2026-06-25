# AGENTS.md — tokenhub-monitor

## 发版

本机是 Windows。`pnpm tauri build` 直接出 msi + nsis 在
`src-tauri/target/release/bundle/`，`gh release create` 手发即可——不打 git tag，
不触发 CI。

只发用户明确要求的平台；本机不能验证的（macOS dmg）不要主动挂。

## 已知坑

- **pnpm-workspace.yaml**：必须有 `packages` 字段，否则 pnpm 9 卡 `setup-node`
  cache 步骤。`allowBuilds` 是错别字段，正确名 `onlyBuiltDependencies: [esbuild]`。
- **`create-release` job**：`needs: [build-macos, build-windows]`，任一 build 失败
  （哪怕 cleanup）都会拖死发版。如果非走 CI，只看 artifact 上没上传，别看 run
  的 overall conclusion。