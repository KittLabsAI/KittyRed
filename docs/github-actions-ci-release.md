# GitHub Actions CI 与桌面安装包发布

## 目标

- PR 和 `main` 提交自动做最小校验。
- 发布时自动构建内置 Python 运行时。
- 产出最终用户可直接下载的 KittyRed 桌面安装包。

## 当前策略

- CI workflow: `.github/workflows/ci.yml`
- Release workflow: `.github/workflows/release.yml`
- Release workflow 现在覆盖 macOS、Windows、Linux 三个平台矩阵

## CI workflow 做什么

`CI` 会按顺序执行：

1. `npm ci`
2. `npm test`
3. `npm run build`
4. `python3 -m unittest backend.tests.test_runtime_bundle`
5. `npm run build:python-runtime`
6. `npm run smoke:python-runtime`
7. `cargo test python::tests`

这里的 Python 校验只针对“内置 Python 运行时构建链路”和“Rust bundled bridge 路径解析”，避免把外网 AKShare 探针引进 PR 校验。

## Release workflow 做什么

`Release Desktop Bundle` 会按顺序执行：

1. `npm ci`
2. `npm run build:python-runtime`
3. `npm run smoke:python-runtime`
4. Linux runner 额外安装 Tauri 所需系统依赖
5. 使用 `tauri-apps/tauri-action@v0.6.2` 构建桌面安装包
6. 上传 workflow artifacts
7. 将产物附加到 GitHub Release

## 如何触发

- 手动触发：GitHub Actions 页面选择 `Release Desktop Bundle`，点击 `Run workflow`
- 标签触发：push `v*` 标签，例如 `v1.0.1`

发布行为：

- `workflow_dispatch`
  - 创建 draft release，适合手动试跑
- `push v* tag`
  - 直接创建正式 release，并附加三平台安装包

## 产物说明

- `src-tauri/resources/python/`
  - 这是打包前的本地 Python runtime staging 目录
  - 已加入 `.gitignore`
- macOS
  - `*.dmg`
  - `*.app`
- Windows
  - `*.exe`
  - `*.msi`
- Linux
  - `*.AppImage`
  - `*.deb`
  - `*.rpm`

## 本地预演命令

```bash
npm run build:python-runtime
npm run smoke:python-runtime
cd src-tauri && cargo test python::tests
npm run build
npm run tauri build
```

## 备注

- 当前方案的目标是“用户下载后无需再安装 Python”。
- 当前 workflow 在手动触发时创建 draft release，在 `v*` 标签触发时直接创建正式 release。
- 当前 workflow 还没有接入 macOS notarization、Windows 代码签名或 Linux 仓库分发签名。
