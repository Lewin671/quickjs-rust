# T056：机器栈相位、启动与 unpack 指令诊断

- Status: diagnostic；无验收结论，无 attempt 消耗。
- Implementation: 生产代码仍为 dfff3bb6；本轮仅诊断生产器、证据和 lead-only 协议。
- Verified at: e5172755 的生产树；canonical f58/8f/dfff 镜像哈希与既有 receipts 一致。
- Evidence: [地址/代码/计数及探针](../diagnostics/t056-stack/README.md)；[lead 两次诊断结果](../diagnostics/t056-stack/layout-results.json)。
- Unresolved: 旧计时进程的实际机器栈相位未记录；本轮布局保真快照不支持“dfff 栈槽移动后碰 bank”的直接解释，但尚未提交栈相位计时。
- Next action: lead 运行已准备的 ≤25min 栈相位探针；满足地址覆盖和精度条件才反证/支持假说。最后一次 attempt 不动。

## 已完成的 lead 诊断

`20261021T084250Z-t056-code-layout-mini` done0：六布局 global C/B
1.424/1.459/1.439/1.740/1.416/1.751，instructions约1.0002。108个A/A格中6个CI过宽，
因此预注册 router 为 **inconclusive**，不能改写为通过或放松AA；大幅global事件在每个测试布局都可见。
local .92–1.00、branch .93–1.01、heterogeneous .87–.89；unpack1.03–1.05、instructions1.0119。

`20261021T085751Z-t056-code-layout-cross` done0，600.242s：global在四个code×zone格为
1.434/1.431/1.442/1.466；zone约1.00、16B代码平移约1.02–1.04，未见解释43%的交互。
local .92–.94；unpack1.03–1.06，zone0使双方约.81，说明其allocator因素很大，
但不能把这个共同加速解释为C/B差距消失。empty C/B约1.04–1.06、instructions约1.10。
这只排除了所测uniform平移/zone的解释，不排除内部load/store PC关系、其他数据流或未测布局。
两作业均diagnostic，attempts=0；原冻结计划、最后一次attempt均未改变。

## 热循环机器栈：保留原调用帧的实测方法

非交互LLDB被系统拒绝调试权限（091115Z）；SIGSEGV子进程报告缺少thread state（091341Z），
二者均不作为寄存器证据，也未修改系统权限。091853Z快照首版因期望near branch而拒绝，未执行该镜像。

有效方法是**canonical镜像副本的终止式跳板**：只把typed执行器第一轮dispatch头10B改为跳转，
在同函数尚未执行的handler区域放短桥。到达桥前，调用者、函数入口、prologue和全部原始路径字节不变；
桥先用`push %rsp`保存进入桥时的原RSP，再保存RBP/R9/R11、原`[rbp-0x30]`、ops指针和长度，
直接write syscall导出56B并exit。它没有logger调用、额外原生帧或运行时分配；临时push发生在保存原RSP之后。
canonical原本未签名；最终快照也保持未签名，整个文件除两个patch区域逐字节相同（含header/长度），
所有映射section地址/长度/文件偏移不变。早期ad-hoc签名副本可能改变启动元数据，不再用于证明canonical相位；
它们的原始记录保留，新renderer只接受unsigned资格。
镜像只跑到第一轮头，**不作计时或语义测试镜像**。保存RSP不是桥降栈后的值。

`20261021T100055Z-t056-stack-unsigned-qualification`：三版本global/local各29个phase、N与2N，共348个样本。
固定active路径、环境、未知空padding键；N=23,068,672/46,137,344均8位；argv字节数、环境各键长度、
条目数及有序hash保留。真实首样本：

| 镜像 | 用例 | 原RSP | RBP | 唯一循环栈读地址 RBP−0x30 | bank |
| --- | --- | --- | --- | --- | --- |
| f58 | global | `0x7ff7b0648db0` | `0x7ff7b0648f50` | `0x7ff7b0648f20` | `0x7f90fd87e400` |
| f58 | local | `0x7ff7b7f93db0` | `0x7ff7b7f93f50` | `0x7ff7b7f93f20` | `0x7f7bec08b400` |
| 8f | global | `0x7ff7bcf97dd0` | `0x7ff7bcf97f60` | `0x7ff7bcf97f30` | `0x7f97b3884000` |
| 8f | local | `0x7ff7bae17dd0` | `0x7ff7bae17f60` | `0x7ff7bae17f30` | `0x7f94e189e000` |
| dfff | global | `0x7ff7b9b90dd0` | `0x7ff7b9b90f60` | `0x7ff7b9b90f30` | `0x7ff6f9084000` |
| dfff | local | `0x7ff7be13edd0` | `0x7ff7be13ef60` | `0x7ff7be13ef30` | `0x7fd8b389e000` |

同环境下N/2N低位一致，8f/dfff也一致。prologue：五个callee-save push共0x28，
f58再`sub rsp,0x178`，8f/dfff均`sub rsp,0x168`，故热头`RBP−RSP`分别0x1a0/0x190。
`[rbp−0x30]`在prologue保存r8（scratch/context指针）；成功数值Binary路径每次读8B恢复r8。
这些循环内不调用native regexp；`call_native_family`等帧大小变化不能直接外推到这条调用链。

没有测kernel入口RSP，而是直接保存目标热头原RSP，因此不靠估算初始栈推导。地址适用于所封存环境，
**不是旧timing进程实址**。最终unsigned资格通过相同`/usr/bin/time -l`父进程确认相位；
job路径/环境和case id的1字节差别足以跨16B对齐边界，不能把固定call depth等同于跨所有启动参数固定相位。

## 哪些访问真的逐轮发生

[更正后的两份原镜像审计](../diagnostics/t056-stack/numeric-audit-dfff.json)覆盖实际Increment。
旧架构审计把Update第二个条件分支误选为Decrement，故17条/81B和−1.0不是此benchmark的真实Update路径。
新审计：Lt32/140B、Exit12/51、Add32/141、**Increment18/86**、Dispatch13/48、Backedge4/22；
三版本的选定路径仍是地址归一化字节等价。旧原始证据保留，未覆盖改写。

| 实际访问 | 宽度/每轮次数 | 地址 |
| --- | --- | --- |
| Binary恢复scratch/context基址 | 8B；global2次，local3次 | `RBP−0x30 = RSP+0x170`(f58)或`RSP+0x160`(8f/dfff) |
| 循环内机器栈store | **0** | prologue保存不是逐轮store；数值路径无push/call/RSP调整 |
| Lt/Add读scalar | tag1B、payload8B；r2/r3、r4/r5，local另r4/r6 | `bank+16*i`与`+8` |
| Lt写r0；Add写r4 | tag1B、第二字节1B、payload8B | bank+{0,1,8..15,64,65,72..79} |
| Increment写r2 | tag1B、payload8B | bank+{32,40..47} |
| Exit读r0 | tag1B、bool1B | bank+{0,1} |
| dispatch重读program字段 | ops指针8B、length8B | `R11+8`、`R11+16`，R11并非机器栈 |

r5(global)与r5/r6(local)为入口seed，不写回每轮；global cell/Realm也不逐轮读。
对上表首样本，机器栈读的f20/f30与bank活跃写区间（base400起、新版本000起）在**逐字节、4B、8B
granule**均不重合。没有循环栈store，因此反向“栈store→bank load”也无逐轮候选。
Intel文档描述store后load的4K部分地址别名及相关事件；这里的4/8B检查是按真实访问宽度作保守筛查，
不是声称公开资料给出了Comet Lake标量比较器的完整精度，也不把AVX示例的固定周期直接套给8B标量。
见[Intel Optimization Manual](https://cdrdv2-public.intel.com/814198/248966-Optimization-Reference-Manual-V1-049.pdf)，
及[Intel PMU说明](https://cdrdv2-public.intel.com/671378/335279-performance-monitoring-events-guide.pdf)。

另一个必须保留的流：新版本global的`program+8`与`bank+8`同为页内008，后者是Lt写的payload，
前者是下一次dispatch读的ops指针。local的program通常600，没有这个重合；**8f和dfff均有该global重合**，
所以它不是两版本差异的现成解释。其store指令PC内部偏移却变了：8f的0x29ca→dfff0x29be，
load位于相同0x8f。这保留了其他数据流/内存依赖预测的调查方向，不能把未见栈冲突等同于排除所有4K问题。

## 栈相位资格与待执行探针

`20261021T100055Z-t056-stack-unsigned-qualification` done0/4s：348个地址样本，29个共同padding长度；
所有组栈槽按`−padding mod4096`移动，N/2N一致；每版本global/local都覆盖实际重合与不重合。
例如dfff global/local均在3824/3856/3888出现8B-granule候选重合；这些是**地址预测，不是wall结果**。
base global/local三项均稳定；8f local有1次program/ops变化，dfff local亦有1次；
8f/dfff global三项均稳定。故“未知env padding不增加引擎分配”不等于“冻结所有独立进程heap地址”。
全部异常保留，不删除、不重试；探针将现场重新记录分布并限制可反证的范围。

[Lead协议](../diagnostics/t056-stack/README.md)：先按现场地址封存共同粗/细网格，最多30点、4对、
global/local/empty、两版本各自A/A；只改变同名未知env值长度，代码/bytecode/zone不变。
≤25min；尚未提交计时。平坦结果只有在预测hit/clean覆盖、候选global热heap低位稳定、AA/指令/CI均合格时，
才反证**该条件下**的栈-bank别名解释；强相关仅支持栈相位敏感性，不自动证明硬件根因或旧natural回退。
若被证实，应评审机器帧内register window与spill的共同物理布局；仅把bank搬到另一个heap arena
并不固定它与机器栈的相对offset。若被反证，不应为这个假说先实施frame/packet迁移。

## empty：新增约1.42M指令在stage A初始化路径

`20261021T092319Z-t056-startup-event-attribution` done0/215s，canonical各3次计数中位数：

| 调用 | f58 | 8f | dfff | dfff/f58 |
| --- | ---: | ---: | ---: | ---: |
| `--raw -e '0;'` | 13,153,115 | 14,573,892 | 14,576,825 | 1.10824 |
| `--help`（不建realm） | 8,183,923 | 8,196,442 | 8,196,744 | 1.00157 |
| whole unpack | 1,028,607,704 | 1,030,266,365 | 1,040,673,852 | 1.01173 |

empty主要增量在8f之前的stage A，不是dfff的112B header修补新加了10%。CLI-only差额仅约12.8K；
新增工作集中于eval后的realm/property初始化与其清理；不能把每条新增指令精确归给某个源函数。
微基准N/2N会消去固定启动成本，故empty增量解释不了global的43%运行期wall。

独立perf-counters归档仅统计事件，**不拿其指令或wall作canonical数据**。
`093246Z-t056-startup-bootstrap-events`在dfff的`initialize_builtins`返回前取事件快照：

| 事件 | builtin安装结束 | empty最终 |
| --- | ---: | ---: |
| atoms创建 / intern调用 | 460 / 637 | 462 / 639 |
| shape创建 / field复制 | 663 / 11,662 | 665 / 11,845 |
| key比较 / add probes | 13,217 / 784 | 13,852 / 787 |
| transition edge misses | 660 | 662 |

几乎全部shape/field工作发生在builtin安装期；base旧属性存储没有这套shape结构（不是把未计数的baseline事件记为零成本）。
通用修复方向是**fresh内建对象的bulk descriptor/layout构建**：复用`PropertyStorage::from_ordered`类接口，
从最终有序描述符一次形成layout和值区，避免每加字段复制一份过渡shape并重复intern/probe。
保留属性顺序、attributes、prototype、唯一binding cell及realm隔离；不共享可变intrinsics或恢复名字快照。
未来原型必须证明启动field复制和canonical指令同时下降，且完整语义/布局控制不过界。

## unpack：少建索引，但扫描与代码图代价持续

8f→dfff whole计数约+10.41M；empty新增量约1.42M只占相对base的unpack新增12.07M的约12%量级，
不能把whole差距全归启动（这不是两个程序可直接相减的精确成本分解）。
同一事件生产器给出：

| unpack事件 | 8f | dfff | 变化 |
| --- | ---: | ---: | ---: |
| key比较 | 764,580 | 830,096 | +65,516 |
| index builds / fields | 468 / 32,894 | 4 / 182 | −464 / −32,712 |
| hashed lookups | 108,954 | 108,025 | −929 |
| atoms / shape / copied fields | 9066 / 1194 / 45656 | 相同 | 0 |

此前[消融](T056-architecture-review.md#unpack-code指令增加的已知归属与边界)已经显示write-index admission alone对8f约1.0098，
header-only约1.0121，adapter-only约1.0002；不可相加。新计数证实跳过transient index增加了扫描，
但不能把全部+1%精确换算成65K次比较；native/regexp内联图也变了。

通用方向：为大批动态追加提供amortized索引/构建器，而非每个短命shape建hash、也非永远全线性扫描；
已链接键沿Atom身份到slot的路径，避免反复字符串比较。用操作/所有权状态决定admission，不能识别unpack源码。
要求同时保住减少的索引分配、降低比较/retired instructions，并审计native服务边界的稳定ABI与内联图。
不能用再改一次header尺寸、挑inline标注或重开已拒绝的add-replay来代替前瞻计划。
这些均是待冻结原型的方向，本轮不实施，不花最后attempt。

## 本轮核验（unsigned修正）

e5172755生产树未变；`./scripts/check.sh`通过（含5170 subset）。更正后的两版本字节审计通过；
unsigned资格完整通过；collector用fake process/counters/phase走完4个block，并检查flat/sensitive/heap-confounded分支、
A/A、env、全部16B相位的几何覆盖、DST和25分钟守卫；[报告](../diagnostics/t056-stack/validation-unsigned.json)不是计时结果。
context、staged fast、whitespace检查通过。无生产代码、依赖、公开API或冻结policy变化。
