# AGENTS.md — tokenhub-monitor

## Release 工作流口径

`release.yml` 的 `create-release` job 依赖 `needs: [build-macos, build-windows]`，
**两个 build job 都成功（且无 cleanup 报错）才会自动发版**。

**规则**：

1. **artifact 一冒出就手发版**：任何平台的构建产物一旦在 CI artifact 里冒出来，
   立刻用 `gh release create` / `gh release upload` 手发版，不要等两个 build 都跑完。
   CI 自动发版只在它历史稳定时才靠；本仓库目前 macOS build 仍脆，不能拖 Windows。
2. **只发用户明确要求的平台**：CI 顺手出了别的平台 artifact 不等于该发。**不要主动
   把自己不能在本机验证的资产挂上去**——本机是 Windows，就只发 Windows 资产；macOS
   资产只在用户明确要、且能在 macOS 上验证后再加。
3. **artifact 在就直接 `gh release create <tag> <asset...>` 或 `gh release upload`**，
   不要无谓等 `create-release` job 触发。

## 已知坑

- **pnpm 9 + pnpm-workspace.yaml**：必须有 `packages` 字段，否则 `setup-node@v4 (cache:pnpm)`
  会卡在 `pnpm store path` 校验阶段。`allowBuilds` 是错别字段，正确名是
  `onlyBuiltDependencies: [esbuild]`。
- **macOS build 步骤**：需要 `RUSTFLAGS: "-A unexpected_cfgs"`（workflow 里已经加了），
  绕过 `objc 0.2.7` 的 `sel_impl!` 宏生成的 `cfg(cargo-clippy)` 警告。
- **setup-rust-lang/setup-rust-toolchain@v1**：用 `target:`（单数），不是 `targets:`。

## 改动/发版后自检

1. `gh run list --workflow=release.yml --limit 1` 看最新 run 的 job 列表，**只看 artifact
   是否上传**（`/actions/runs/<id>/artifacts`），不要被 run 的 overall conclusion 误导。
2. artifact 在就直接 `gh release create <tag> <asset...>` 或 `gh release upload`，
   不要无谓等 `create-release` job 触发。