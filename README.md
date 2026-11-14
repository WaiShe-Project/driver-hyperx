# driver-hyperx

WaiShe 的 HyperX 设备驱动：用户态的协议适配器，由 WaiShe Bridge 加载。不是操作系统驱动。

**当前阶段：0.1.0，Cloud III Wireless（接收器 `03F0:0FCC`）已在 WaiShe Bridge 中实机验收（M1，2026-11-14）。** 构建：`cargo build --release --target wasm32-unknown-unknown`；测试：先构建再 `cargo test`；打包：用 `waishe-driver-sdk` 的 `waishe-driver-dev pack`。

## 设备支持

下表是协议层面的实机验证结果（2026-11-14），不代表已有可用驱动。

| 设备 | 连接方式 | 电量 | 充电状态 | 连接状态 | 麦克风静音状态 |
| --- | --- | --- | --- | --- | --- |
| Cloud III Wireless | 2.4G 接收器 `0FCC` | verified | verified（仅“未充电”） | verified | verified（查询与按键上报） |

验证等级：`verified` 实机验证 / `simulated` 回放测试 / `inferred` 同协议族推断 / `broken` 已知不可用。

每款设备有一份验证计划与记录：

- [Cloud III Wireless](docs/cloud-iii-wireless.md)

## 范围

首期只读：电量、充电状态、连接状态、麦克风静音状态。

不做：修改任何耳机设置、软件静音控制、音量、EQ、音效、固件升级。其中软件静音与音量属于后续增强，固件类命令永远不在范围内。

麦克风静音状态依赖耳机主动上报，本驱动需要持续监听接收器的输入报告。监听不得影响音频流和耳机实体按键。

## 使用注意

HyperX NGENUITY 会与本驱动同时访问接收器，双方都会收到对方查询的应答。M0 实测中 NGENUITY 运行时读取正常，见设备文档。

## 协议来源

见 [PROVENANCE.md](PROVENANCE.md)。任何协议代码合并前必须先在其中登记来源。

## 许可证

[MIT](LICENSE)。

本驱动预期参照 HyperBatteryMonitor（MIT）。为保持 MIT，本仓库不得引入来自 GPL 或 AGPL 项目的代码。

## 相关

- 驱动接口与能力模型：`waishe-driver-sdk` 仓库。
- 项目总览：`waishe` 仓库。
- 贡献流程：`.github` 仓库的 `CONTRIBUTING.md`。

## 声明

本项目与 HyperX、HP Inc. 没有关联，也未获其认可或赞助。HyperX 及相关产品名称是其所有者的商标，仅用于指明兼容对象。
