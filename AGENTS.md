# KittyRed AGENTS Guide

## 项目形态

- KittyRed 是本地 A 股模拟投资助手。
- 前端在 `src/`，使用 React、Vite 和 TypeScript。
- Tauri 命令、本地服务、SQLite 持久化和业务逻辑在 `src-tauri/src/`。
- Python AKShare 适配层在 `backend/`，经 `python3 -m backend.akshare_service` 被 Rust 命令调用。
- 当前代码、共享类型和测试是本地事实来源；不要按旧产品或其他市场产品假设开发。

## 数据源和缓存

- 外部行情入口只允许 AKShare。不要新增真实券商、交易所、加密货币、中心化交易平台或其他市场数据入口。
- 实时行情优先使用 AKShare 的雪球单股接口，内部 source 可以标记为 `akshare:xueqiu`。
- K 线、交易日、股票池搜索继续使用 AKShare 相关接口。
- AKShare 是行情数据源，不是交易引擎；模拟交易价格可以来自行情，但不要把 AKShare 当作下单系统。
- 自选股行情写入 SQLite `market_ticker_cache`；`list_markets` 只读自选股缓存，页面先展示缓存，再后台刷新。
- K 线写入 SQLite `market_candle_cache`；K 线拉取失败时允许回退缓存。
- A 股股票池写入 SQLite `a_share_symbol_cache`；搜索应优先读本地缓存，避免每次输入都实时拉全量股票池。
- 自选股行情后台刷新间隔是 60 秒；股票池后台预热间隔是 1 小时。
- 自选股行情刷新失败时保留旧缓存，不清空页面，也不要返回会让页面误判为无数据的空结果。

## A 股范围和中文 UI

- 产品只关注沪深 A 股和本地模拟交易。
- 用户可见 UI 文案、按钮、空态、表头、错误提示、设置项必须使用中文。
- 代码、测试和文档不要引入美元稳定币计价、真实券商账户、交易所账户、衍生品、跨市场套利或其他市场叙事。
- 保持模拟账户边界清晰：当前版本不连接真实账户，不提供实盘交易能力。
- 前端非 Tauri 预览路径也要保持 A 股和中文语义，不能退回旧产品样例。

## 开发边界

- 保持 Tauri commands 薄；行情、推荐、模拟交易、组合、助手、设置和信号逻辑放回对应业务模块。
- 前端桥接逻辑集中在 `src/lib/tauri.ts`、`src/lib/settings.ts`、`src/lib/akshare.ts` 和相关类型文件；不要在页面组件里散落 raw `invoke()`。
- 修改命令、DTO 或共享类型时，同步更新 Rust models、TypeScript types、调用方和测试。
- 不要手改生成物或依赖产物：`dist/`、`node_modules/`、`src-tauri/target/`、`src-tauri/gen/schemas/`。
- 只做用户要求的最小改动；不要顺手重构无关代码。

## TDD 和验证

- 修 bug 时先写能复现问题的失败测试，再实现修复。
- 新功能先写目标行为测试，再实现代码。
- 文档或纯配置变更可以不新增代码测试，但要运行能证明约束的最小检查。
- 前端测试通常在 `src/features/**/*.test.tsx`、`src/components/**/*.test.tsx`、`src/lib/**/*.test.ts`。
- Rust 测试多数在 `src-tauri/src/**` 内联。
- 优先运行最小相关检查：`npm test -- <file>`、`python3 -m unittest <module>`、`cd src-tauri && cargo test <module>`。
- 影响 TypeScript 类型、构建入口或跨层 DTO 时，最后运行 `npm run build`。

## 高价值入口

- UI 桥接：`src/lib/tauri.ts`、`src/lib/settings.ts`、`src/lib/akshare.ts`、`src/lib/types.ts`
- AKShare 适配：`backend/akshare_adapter.py`、`backend/akshare_service.py`、`src-tauri/src/market/akshare.rs`
- 缓存和行情服务：`src-tauri/src/market/cache.rs`、`src-tauri/src/market/mod.rs`、`src-tauri/src/commands/market.rs`
- 启动和后台刷新：`src-tauri/src/app_state.rs`
- 页面入口：`src/router.tsx`、`src/App.tsx`、`src/features/*`
