# AudioScript — 配色与视觉语言

**日期** 2026-10-16 · 落在 `src/shared/theme.css` 与 `src/ui/`

> 架构与行为规范见 [SPEC.md](./SPEC.md)。

---

## 1. 色板

深色为参考截图实测值（@2x 采集，**所有 CSS 值 = 截图像素 ÷ 2**）；浅色取 Apple 浅色模式中**同名**的系统色（systemBlue / systemGreen / …），不是自行调的近似值。

| 令牌 | 深色 | 浅色 | 用途 |
|---|---|---|---|
| `--as-surface-0` | `#1e1e1e` | `#ffffff` | 内容区底 |
| `--as-surface-1` | `#2a2a2a` | `#f7f7f7` | 卡片 / 分组容器 |
| `--as-surface-2` | `#333333` | `#e2e2e2` | 悬停、凹陷 |
| `--as-surface-sidebar` | `#2a2a2a` | `#ececec` | 边栏。**必须独立成档**，见硬规则 5 |
| `--as-surface-selected` | `#192b3a` | `#dce9f9` | 选中卡片底（实心，不是 `surface-1` 加透明度） |
| `--as-ink-0` | `#ffffff` | `#1d1d1f` | 标题 |
| `--as-ink-1` | `#999999` | `#6e6e73` | 正文 / 说明 |
| `--as-ink-2` | `#7a7a7a` | `#8e8e93` | 提示 / 脚注 |
| `--as-accent` | `#007aff` | `#007aff` | systemBlue。深色下**不是** `#0A84FF` |
| `--as-success` | `#30d158` | `#34c759` | systemGreen |
| `--as-danger` | `#ff453a` | `#ff3b30` | systemRed |
| `--as-warn` | `#ff9f0a` | `#ff9500` | systemOrange |

底色是**中性灰**（三通道相同，彩度为零）：彩色只出现在主操作（实心绿/红 + 边缘流光，§3）、可选的悬浮窗流光边框与选中/焦点的蓝。`logo.svg` 用的是同一套色——改一个要顺手看另一个。

关键几何：

| 元素 | 值 |
|---|---|
| 卡片圆角 | 10px |
| 卡片边框 | 2px（选中），未选中为透明 |
| 卡片间距 / 内边距 | 10px / 16px |
| 徽章 | 高 22px / 圆角 4px |
| 按钮圆角 | 6px（主操作为胶囊） |

## 2. 硬规则

由 `theme.css` 顶部注释与组件文件承载，**改动前必读**：

1. `.app-window.theme-light` **只作用于主窗口**。`overlay.html` 没有 `app-window` 类，悬浮窗**永远**拿 `:root` 的深色值——字幕浮在视频画面上，浅色没法读。
2. `html:has(.overlay-body)` 那三条是透明窗的命脉。HTML/body MUST 完全透明、MUST NOT 有滚动条。验证：`getComputedStyle(document.body).backgroundColor` 必须是 `rgba(0, 0, 0, 0)`。
3. `--radius-panel`（8px）的值 MUST NOT 改——悬浮窗的光圈与虚线框按它调的。主窗口卡片另起 `--radius-card`（10px）。
4. `@theme static` 的 `static` MUST 保留。缺了它，`[var(--color-x)]` 这类任意值写法会**静默失效**——颜色变成继承或透明，不报错。
5. **边栏与卡片 MUST NOT 共用令牌**：两者在深色下恰好同值（`#2a2a2a`），一旦主题翻转就分道扬镳——共用会让浅色下的高低位关系反过来（边栏变白、内容区变灰）。
6. 手绘字形（键盘 / GitHub / 边栏）MUST 传入与相邻图标相同的 `strokeWidth`——不传会**静默**变细（`undefined` 覆盖掉 Heroicons 写死的 1.5，SVG 默认值 1 生效）。

## 3. 组件语言

- **图标入口唯一**：`src/ui/Icon.tsx`，注册表按**语义名**索引，`navigation.ts` 与各页面只认语义名——换图标库时一行都不用改。手绘字形 MUST 画在同一套尺子上（24 格、1.5 描边）。
- **主操作（`PrimaryAction`）**：胶囊圆角，默认 `--as-success` 实心绿（绿=开始），采集中换 `--as-danger` 实心红（红=停止）。按钮边缘叠一圈跟指针缓动的流光（Google 全色板 `conic-gradient` 色盘 + conic 遮罩 + 横向拉伸 + 模糊），静止 40%、悬停 100%，色盘比遮罩慢半拍形成流动。全应用**仅有两处**整块铺色的控件：它，与欢迎页的 `Button variant="cta"`（同为实心绿，同「开始」语义）。其余按钮一律描边或纯文字；`--as-accent` 只留给选中态、焦点与编辑标记，MUST NOT 挪回来当主操作底色。
- **悬浮窗彩色边框（`glow`，默认关）**：与主操作共用 `src/shared/glow.ts` 的同一套全色板 `conic-gradient`，裁成 2px 描边。**流光速度（`glowSpeed`，0 ~ 100，默认 0）** 大于 0 时，采集中色盘按该速度绕边流动，空闲时静止并降到半透明；为 0 时描边始终静止且保持全亮。MUST NOT 用外发光——窗口边界会把它裁掉。
- **卡片分层 MUST 靠纯色差**，MUST NOT 描边或投影；边栏与内容区之间同理 MUST NOT 加分隔线。
- **分组卡片里的行间分隔线**：左右各内缩 16px（与行内 `px-4` 对齐）+ `opacity-60`。MUST NOT 用 `divide-y`——它的线撑满整行宽，会顶到卡片圆角上。
- **电平表（`LevelMeter`）**：5 颗 6px 圆点，亮 `--as-success` / 灭 `--as-surface-2`；会话采集中与「音频」页预览时刷新（200ms，SPEC §6.1）——离开页面或主窗口隐藏即停。映射按 dBFS（-60…-6）而不是线性 RMS——线性乘出来永远只亮第一颗；回落包络只为了不让它闪。
- **输入设备选择器（`DeviceSelect`）**：自定义浮层，MUST NOT 用原生 `<select>`——`<option>` 渲染不了动态电平。每个选项是「5 点电平 + 设备名 + 声道/采样率」，展开时显示全列表电平，收起时只留选中设备。浮层 MUST 用 portal 渲染：分组卡片带 `overflow-hidden`，就地绝对定位会被裁掉。Mix 模式下拆成「麦克风」「系统声音」两行，各自一个选择器，MUST NOT 用一个下拉表达两个选中值。
- **`ChoiceCard` / `ChoiceGroup`**：选中框 MUST 用 `border-2` + **未选中时透明**，MUST NOT "选中才加边框"——边框必须内嵌，否则整组会抖一下。MUST 走 `role="radio"` / `aria-checked`，MUST NOT 用 `<button aria-pressed>`（互斥选项，读屏器要能念出"共 3 项、第 2 项"）。
- **`StatusBadge`**：各档**同一个形状**（淡底 + 同色系深字），没有实心档——颜色只承担语义，不承担音量。参考图的实心绿在一屏三颗、淡灰底上是整页最响的东西，而它表达的是"一切正常"。
- **`SidebarToggle`（暂时下线）**：组件与入口已移除，边栏恒显。重做时按原来的两条：收起后 MUST 在内容区左上角**重新渲染一个**入口，MUST NOT 只把边栏 `display:none`——跟着一起消失就再也打不开了；可见性 MUST 持久化在 `Preferences.sidebarVisible` 而非只存前端（「重置所有设置」得能把它清掉）。
- **状态点（`PrimaryAction` 里 6px 的圆点）**：就绪 `--as-success`、启动 / 停止中 `--as-warn`（脉冲）、采集中 `--as-danger`（脉冲）——与主操作按钮同一套绿 / 红语义。圆点颜色与它右边的文案 MUST 同源，分开判断就会出现"橙点 + 就绪"。
- **`PermissionPanel`**：欢迎页与「通用 → 权限」区块**共用同一组件**，MUST NOT 各写一份（文案与状态映射分叉是这块最容易发生的事）。「没测过」用 `muted` 而不是 `warn`——颜色承担语义，不承担音量。系统音频的检测结论**只活在组件状态里，MUST NOT 落盘**：昨天缓存的「可用」是谎话。

