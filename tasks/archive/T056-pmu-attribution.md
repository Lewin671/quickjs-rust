# T056：栈假说关闭、浮点状态与 PMU 访问诊断

- Status: diagnostic；无验收结论，无 attempt 消耗。
- Implementation: 生产树仍为 dfff3bb6；仅证据、只读诊断生产器和 lead-only 采集协议。
- Verified at: 2ec4b673 生产树；f58/8f/dfff canonical 哈希及 receipts 复核；本轮非计时队列全部结果见证据目录。
- Evidence: [协议、生产器及 JSON](../diagnostics/t056-pmu/README.md)；[上轮代码/栈/启动诊断](T056-stack-phase-diagnosis.md)。
- Unresolved: iMac 当前没有可用的非交互 PMU 入口，硬件根因未归属；没有采集 cycles/assist/stall 事件。
- Next action: lead 提供已有授权的 PMU 工具入口后执行 ≤20min 协议并审计真实事件；停止廉价 placement 猜测，不实施生产修复，保留最后一次 attempt。

## 栈-bank 别名假说关闭

lead 的 `20261021T101153Z-t056-stack-phase-probe` done0/1049.23s，30相位、4对、
1440个采样单元，全部输出一致。global dfff/base 在29相位为1.415414–1.447996，
另一个3904B相位为1.228018；local为0.930127–0.956069，指令基本相同。
base global collision/clean=0.997951，CI[0.997368,0.998081]；base A/A接近1。
两版本global的bank/program/ops低位全稳定；dfff local的program/ops有变化，不能称所有组都稳定。

**按 lead 的诊断判断，初始栈相位导致栈-bank 别名、解释global约43%回退的假说被反证，停止据此挑offset或设计修复。**
这与原自动评估的范围分开：候选global在7相位的A/A精度未通过，collision/clean CI很宽，
自动评估仍为 `inconclusive`、primary_eligible=false；没有改写原规则、筛掉3904异常或宣称验收通过。
[完整相位中位数、原评估及文件哈希](../diagnostics/t056-pmu/stack-result.json)全部保留。
结合K=6、16B平移和malloc zone结果，现有数据不足以指出硬件事件；不再追加廉价placement猜测。

## 标量不是 subnormal、NaN 或另一种 Typed 变体

非计时 `20261021T104647Z-t056-fp-state-v3`：三版本、global/local、四个回边检查点，共24快照。
typed入口之前解释器已跑一轮，因此checkpoint0时i=1；不是将入口期望改成i=0。
下表是global r2/r4；三版本全部一致（十六进制为f64 payload，不是tagged值整体）：

| checkpoint | r2=i 与 r4=sum | payload bits |
| --- | ---: | --- |
| 0 | 1 | `3ff0000000000000` |
| 1 | 2 | `4000000000000000` |
| 1024 | 1025 | `4090040000000000` |
| 11534336 | 11534337 | `4166000020000000` |

r3=N=23,068,672，bits `4176000000000000`；r5=1.0，bits `3ff0000000000000`。
r2..r5的tag均0=`Typed::Number`，payload均normal，无Int、NaN-boxed或subnormal差别。
local另有r6=2.0，sum=3*i；所有操作数与结果是远低于2^53的精确整数。
所有快照MXCSR=`0x1fa0`：denormal标志清、FTZ/DAZ均关；precision sticky位5在入口已置位，不能说所有异常标志为零。
因此没有数值异常解释回退的证据，**不能据此声称FP_ASSIST硬件计数为零**。

三版本全为同一 `typed_loop::execute::run<...WideLoopFrame>` 符号；base入口0x10025bd90，
8f/dfff入口0x100007d90。global操作为Lt(r2,r3)→Exit(ip27)→Add(r4,r5)→Increment(r2)；
local多一次Add(r4,r6)，Exit ip35。比较的是枚举有效字段，忽略Rust padding脏字节。
六次完整诊断运行均只有一个TLRUN、结果Ran、无TLDEOPT；对应region为9..26与13..34，
该op列表通过正常Exit结束。未修改canonical另行跑完，stdout/checksum与诊断镜像一致。
入口计数/退出日志来自独立counter诊断镜像；canonical符号与快照分别核验，未把诊断镜像当canonical计时。

快照在原未签名镜像副本上改6B头与未执行handler区域的桥；无新原生调用帧、无分配、无FP算术。
所有其他文件字节、header、section地址/长度保持原样，不重签名。检查点增加整数分支，故不能用于性能归因。
原始操作字节、24份寄存器、MXCSR、变更范围与镜像哈希均记录。
v1入口计数假设失败、v2 Mach-O标签relocation导致checkpoint失败，均不作为有效快照；v3修复并通过。

## iMac 上实际可用与不可用的计数入口

只读检查 jobs：103341Z capabilities-v2、103456Z catalog、104203Z rusage-smoke、105828Z extra-capabilities。
硬件iMac20,1 / i7-10700 / family6 model165(A5) stepping5，macOS12.6。

| 入口 | 当前观测 | 可支持的结论 |
| --- | --- | --- |
| xcrun xctrace | exit72；系统/home Applications无Xcode app，只有CLT | 未找到可运行的Instruments计数器入口 |
| kperf/kperfdata | dylib及符号可加载 | 不代表有计数器权限 |
| kpc_get_thread_counters / force_all_ctrs_get | -1，errno1 EPERM | 当前调用被拒绝；未调用任何setter |
| sudo -n id / sudo -n -l | exit1，需要密码 | 没有现成非交互root入口 |
| csrutil status | SIP enabled | 未修改SIP、权限、服务或系统设置 |
| proc_pid_rusage V4 | 自身及自己创建的zombie可读正数 | 现有cycle-trust探针false，不能用这些cycles算CPI |

[kpc_demo](https://gist.github.com/ibireme/173517c208c7dc333ba962c1f0d67d12)研究只用于确认方案：
self-thread getter不能直接归给exec后的另一个进程；外部进程采样方案涉及特权kperf/kdebug。
没有运行其中的全局reset、安装root工具或冒充取得授权。
[Apple XNU接口](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_kpc.c)与
[rusage定义](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/resource.h)是API依据。

事件字典也不能盲用：Intel的[CPUID映射](https://raw.githubusercontent.com/intel/perfmon/main/mapfile.csv)
把A5映射到[SKL V59](https://raw.githubusercontent.com/intel/perfmon/main/SKL/events/skylake_core.json)。
本机Apple `ASSISTS.FP` 元数据为C1/02，Intel `FP_ASSIST.ANY` 为CA/1E、CMask1（assist cycles），
不能把两者视作同一事件/单位；MACHINE_CLEARS.COUNT需要CMask1/Edge1，STALLS_TOTAL需要CMask4。
这些modifier在所读Apple简表不完整；这不是已证明当前系统编程错误，而是必须核验实际配置。
[event-plan](../diagnostics/t056-pmu/event-plan.json)封存所请求事件、来源SHA和编码差异，尚未编程。

## 可交付的测量协议与核验

[README](../diagnostics/t056-pmu/README.md)给出lead提交与watchdog命令。三canonical精确哈希绑定，
base/8f/dfff/base A/A × global/local × N/2N × 2block，32份raw trace；collect1080s、
body1140s、runner起点≤1200s，队列等待不计，夜间窗及quieting纳入守卫。
本机缺工具时在任何engine launch前写blocked、零PMU样本；无schema/空表/权限/超时均保留原始数据后停止。
尚未做实际xctrace端到端验证，不能保证模板包含全部事件或20min内完整导出；部分采集不得充作结论。
采集完成也明确 `hardware_attribution_validated=false`，需人工核验PID、单位、multiplexing、编码与覆盖。

本轮无生产代码、依赖、公开API、冻结计划/预算变化；最后一次attempt未花。
在2ec4b673的生产树加本提交证据/协议上：`validate.py`通过，复核三个实际canonical文件哈希、
冻结correctness/86-case receipts、24快照/六条trace、32份模拟采集与拒绝分支、deadline/DST。
`./scripts/check.sh`通过（Rust/Python及Test262 subset 5170/5170）；context、文件大小、
staged fast与whitespace均通过。无runtime/parser/lexer语义变化，fast未选择额外语义slice。
模拟PMU输出只测控制流，绝非硬件证据；完整验证清单见validation.json。
