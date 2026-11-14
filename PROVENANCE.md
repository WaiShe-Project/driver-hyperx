# 协议来源记录

本文件登记本仓库中协议知识与代码的来源。规则：

1. 协议代码合并前，先在这里登记来源。没有登记的不合并。
2. 移植或改写了别人的代码，要写明来源文件和提交哈希，并保留原版权声明。
3. 只参考了文档或行为、没有照着源码写，也要登记，注明“仅参考”。
4. 来自自己的抓包，要写明设备、固件版本、抓包环境，并把去除了可识别信息的记录放进 `captures/`。

## 来源

| 来源 | 许可证 | 状态 | 使用方式 | 涉及文件 | 来源版本 |
| --- | --- | --- | --- | --- | --- |
| [HyperBatteryMonitor](https://github.com/davidsantanaalves/HyperBatteryMonitor) | MIT | 已用于取得一个接收器 ID 与功能支持情况，尚未引入代码 | 仅参考：README | `docs/cloud-iii-wireless.md` | 2.4.1，2026-11-14 读取 |
| [HaloBattery](https://github.com/HeyOkay/HaloBattery) | 未核对 | 已用于取得接收器 ID 与协议线索，尚未引入代码 | 仅参考：`docs/protocols.md` | `docs/cloud-iii-wireless.md` | 主分支，2026-11-14 读取 |
| [HyperHeadset](https://github.com/LennardKittner/HyperHeadset) | 未核对 | 未直接读取。HaloBattery 文档称其 Cloud III Wireless 条目取自该项目 | — | — | — |

## 注意

- HyperBatteryMonitor 的 README 说明其早期开发参考过另一个项目，相关声明保存在它的 `THIRD_PARTY_NOTICES.md` 中。移植其代码时须一并核对并保留这些声明。
- HaloBattery 与 HyperHeadset 的许可证确认之前，只能参考其文档中的协议事实，不复制代码。若许可证为 GPL 或 AGPL，则其代码不能进入本仓库。

## 许可证兼容性

本仓库为 MIT。可以引入 MIT、BSD、Apache-2.0 等宽松许可证的代码并保留其声明。不能引入 GPL、AGPL 代码。
