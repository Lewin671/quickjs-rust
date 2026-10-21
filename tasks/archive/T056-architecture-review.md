# T056：解释器热状态与布局稳健性评审

- Status: architecture review；未实现目标架构，未消耗验收 attempt。
- Implementation: 本轮只增加文档和隔离诊断生产器；生产代码仍是 dfff3bb6。
- Verified at: b1ecbaf6 的生产树；base=f58c696e、aligned=8f7d97c7、allocation=dfff3bb6 的独立 iMac 诊断见下表。
- Evidence: [机器可读地址与字节审计](../diagnostics/t056-architecture/address-evidence.json)、[复现及 lead 探针](../diagnostics/t056-architecture/README.md)。
- Unresolved: global_read 的 42.7% wall 回退尚无硬件因果归属；不把低 12 位相同当作证明。
- Next action: lead 先跑已准备的诊断K=6 mini-panel；按预注册规则决定是否考虑在未改dfff上用最后attempt，或转代码×数据2×2。无生产修改、无owner timing提交。

## 判断与阶段决定

**等指令布局回退的主架构方向是代码/构建侧的小内核隔离；数据 packet 留给M2。**
独立评审（lead转交dsh/deepseek-flash意见）与本次补查支持这个优先级。
不能把单次natural triage的1.427当作跨布局持续回退，也没有证据要求先实现HotExecutionPacket。
下一步用未改dfff对base的诊断K=6辨别布局敏感性；它不是acceptance，不消耗attempt。
数据/allocator确有影响，但[既有local_read zone对照](T056-m1-stageA-aligned-scratch.md)约1.070对1.005，不能拿来解释global的43%。

最新 lead 链：`20261021T060648Z-t056-m1a-allocation-correctness` 完整 exact 42,672/42,672、
NG255/subset5170/full pass；`20261021T062918Z-t056-m1a-allocation-pregate86` 86/86；
`20261021T063231Z-t056-m1a-allocation-triage` 为 **likely loss**、diagnostic、attempts=0。
相对 f58：global wall **1.4271 [1.4197,1.4376]**、instructions **1.0002**；
local .948、branch .949、heterogeneous .898；unpack wall **1.0381 [1.0247,1.0688]**、
instructions **1.0119**。这些是 lead 的原镜像结果，不是本轮带日志镜像的 wall。

Stage A仍未集成。先执行[预注册诊断](../diagnostics/t056-code-layout/README.md)：
若global excess在至少5/6布局消失，lead可考虑把最后attempt用于**未改dfff**，仍须原冻结计划的完整门槛；
若持续则不花attempt，先代码×数据2×2；噪声/混合结果也不支持直接花attempt。
不删除失败控制，不调整旧band，不改写K=6历史。有效attempt1/2已用，A+B共剩一次。
若需要内核实现，另冻结前瞻架构单元，不能在旧阶段无限调参。

## 热状态所有权：已有连续窗口，不是从零开始

源码根为 `crates/qjs-runtime/src/`。下表“跳数”从执行器已经取得的基址算起：0 表示
base+index 的数据访问，不代表没有访存；逻辑所有者路径另列。编译器把基址 spill 到机器栈时还会多读，
不能仅凭 Rust 引用数断言实际 load 数。尺寸依据[base/8f类型清单](../diagnostics/t056-architecture/hot-type-sizes.json)（这些类型定义在dfff未变）及本轮实测，为 x86_64：Value/Typed=16、align8；
Vec 头=24、Rc/Box 指针=8；指针容器align8。Rc 另有计数头，表中 payload 大小不含它及 allocator rounding。

| 状态、所有者与源码 | 分配/布局 | 热访问与跳数 | 地址保证 |
| --- | --- | --- | --- |
| wide 寄存器和 operand 窗口；`compact_fn/wide/activation.rs::FrameStorage` | 一个 `Vec<Value>`，locals 与 operand 是同一向量内的片段；另一个 `Vec<WideFrame>`，frame=64/align8 | wide 每 op 读写窗口，0；窗口 base 在帧入口计算，不能每 op 重新解 FrameId | 调用期间可借用；增长会迁移；容量、TLS recycler 历史决定 malloc 位置 |
| numeric compact 寄存器；`compact_fn/activation.rs` | 独立 driver 的连续 Vec 和 frame Vec，TLS 池回收 | 相同的入口窗口规则；不是 wide 共用一个引擎栈 | 与 wide、general、typed 之间没有共同布局所有权 |
| typed scalar bank；`typed_loop/mod.rs::RegisterFile` | Box 内 `[Typed;256]`=4096；base align8，8f/dfff align4096；owner 8 | 数值运算、tag、payload 读写，0；会读取 RIP 常量/跳表 | 仅 bank 内偏移和新版本页对齐 deliberate；它与 ops/机器栈仍各自分配 |
| typed boxed/receiver/sloppy-write bank；`TypedLoopScratch` | scratch=80/align8：bank 指针+三个 Vec 头；boxed 的 Value16，receiver 的 Rc8；sloppy 元素72/align8 | 需要 boxed/object 的 op 才读；Vec base 已取为0，Value→Rc payload 再1，后续 storage 再1或更多 | scratch 按 TypedLoopProgram 池化；三个 backing 独立 malloc，可复用/增长；数值控制中容量0 |
| typed program；`TypedLoopProgram` | 本次 payload528/align8，ops `Vec<TypedOp>`（16/align4），sites Vec（12/align4），constant registers（24/align8）、names、helper 图等分别分配 | 热循环主要读 ops/pc，0；sites/deopt 元信息及 constants 多在入口/退出；不是每轮扫描整个 program | program 挂在 bytecode 的 prepared cache；活期有所有权，但 backing 相对地址 incidental |
| typed shape cache；program 的 `RefCell<Vec<ShapeWays>>` | 独立 Vec；ShapeWays80/align8，每 ways 多个 shape/slot/descriptor 反馈，大小随 ways 类型 | 属性 op 才读；program→Vec 取得 base 后0，shape Rc 标识比较可不解引用；验证/对象 payload 再1–3 | 语义失效有规则；物理邻接无规则；数值控制为空，不是热冲突流 |
| wide program、反馈；`wide/mod.rs::WideProgram` | WideProgram304/align16；ops `Vec<WideOp>`（8/align4）；read site24、write site80、global site32（均align8），cache state272/align8；这些side table是独立Vec/global Box，call 等 cache 另有 Rc/RefCell | 每 op 读 ops，0；named site→cache Rc state→way，通常1–2，再到对象/slot；global cache hit 到 Upvalue 再1 | struct 中放 Vec 不等于 backing 共址；Bytecode 活期稳定，反馈 backing 独立 |
| general Vm；`vm.rs`、`vm_frame.rs`、`vm_bindings.rs` | 原生栈上的 Vm1024/align16、FrameState496/align16；OperandStack32/align8，内含Vec及recycler Rc；local `Vec<Option<Value>>`、local-upvalue Vec、received-upvalue Vec；cold frame Box | local/operand base 后0；slot分类/捕获路径读 authoritative cell，1；兼容 env 访问更多 | 与 compact stack 分离；general fallback 仍可建嵌套 Vm/Rust 调用；`frame_stack.rs` 调度器尚未成为一般调用路径 |
| binding/global cells；`function/upvalue.rs`、`function/env.rs` | 每 cell 一个 `Rc<RefCell<Value>>`；realm binding map 只存同一 cell 身份；private local 未捕获时直接 slot | received-cell 数组→cell payload 1；global name/map/cache 解析另算；优化区域可入口 seed | cell payload 活期稳定；创建先后、分配类和邻居 incidental；删除/访问器/重声明须按现有 generation/marker 失效 |
| realm/env；`CallEnv`→scope Rc→RealmState Rc | 本次 CallEnv192、scope32、Realm1008(base)/1016(8f/dfff)，align8；bindings payload80另 Rc | env入口已有引用，scope/realm0–2跳；内建原型、lookup、native服务按 op 用；不是所有 op 每轮访问 | realm 身份稳定，物理位置/内部 map backing incidental |
| 原 bytecode、常量池；`ir.rs::Bytecode` | 本次 Bytecode1280/align16，Rc/借用 FrameBytecode；code、constants `Vec<Value>`、locals/names、prepared programs 等独立 backing | general 读原 op；wide/typed 读 lowered op，不同时热扫三份；constant load 取pool base后0，引用值再1+ | immutable code 所有权明确，跨 pool/feedback/stack 无相对布局约定 |
| JS 对象、字符串、regexp；`value/object.rs`、`value/js_string.rs` | ObjectData112/120/112，align8，Rc 另16；StringData72、Rc请求88，text/matcher view 各自 backing；property slots/layout/cache分离 | 属性/字符串/native 路径读取，指针链通常1–3以上；unpack 尤其重要 | 稳定 Rc 身份不等于 deliberate 邻域；形状/layout 变化会改变整个 allocator 历史 |
| 机器状态与代码常量 | native frame spill、LLVM 只读常量、dispatch jump table、热基本块 | RIP 常量和跳表也走数据缓存；callee-save/spill 可每轮访问 | ABI/LLVM/链接器决定；不受页对齐 Box 的布局所有权保护 |

依据入口/循环/回收读写点：`typed_loop/execute.rs`（seed、execute、save、recycle），
`typed_loop/frame.rs`、`wide/loop_frame.rs`（slot/cell）；`wide/activation.rs::run_frames`
（借用一次窗口、返回前回收）。**不可把“已有一个 Vec”当作新方案**；待解决的是跨 tier、
执行元数据和借用生命周期的共同布局，以及与冷运行时类型隔离的执行内核。

## 实址、活跃范围和能排除什么

非计时作业 `20261021T072833Z-t056-architecture-capture-v3`：三个源码 archive、标准 fat16/abort/
agents recipe、两独立进程；global 用实际 triage N=23,068,672 与2N=46,137,344，local 用1,000,000。
日志在 typed 返回后、回收前；wide 日志在 run_frames 后。未增加布局字段、allocator wrapper、FFI、unsafe。
但日志改变代码/机器栈，先前日志也可能改变后续 malloc 历史；**这些是诊断镜像地址，不是原计时镜像实址**。
首次地址最接近未记录日志的分配序列。全部值和两次重复在 JSON；下面为 repeat0/N。

| 镜像/用例 | scalar（低12） | typed ops（低12） | wide locals / operand | Realm payload |
| --- | --- | --- | --- | --- |
| base/global | `0x7fa36287e400`(400) | `0x600002914c00`(c00) | `0x600003b14b00` / `0x600003b14b70` | `0x7fa362858610` |
| 8f/global | `0x7fcb2a084000`(000) | `0x6000031ff600`(600) | `0x6000023fb000` / `0x6000023fb070` | `0x7fcb2a013610` |
| dfff/global | `0x7fa294084000`(000) | `0x600001f95900`(900) | `0x600000d9f000` / `0x600000d9f070` | `0x7fa294013610` |
| base/local | `0x7fe3f688b400`(400) | `0x600001884c00`(c00) | `0x600000a84b00` / `0x600000a84b80` | `0x7fe3f6858610` |
| 8f/local | `0x7fb48489e000`(000) | `0x60000175b600`(600) | `0x60000055f000` / `0x60000055f080` | `0x7fb484813610` |
| dfff/local | `0x7fefcd09e000`(000) | `0x600000c71900`(900) | `0x600001e7b000` / `0x600001e7b080` | `0x7fefcd013610` |

global 的 typed ops 只有 `Lt r0,r2,r3; Exit r0; Add r4,r4,r5; Update r2`，
local 多一个 `Add r4,r4,r6`。global 的 `program.global_reads` **为空**；名称解析后的 received
local slot 在入口经 `local_slot_value` 从 Upvalue 装入 r5。每轮既不读 global cell，也不读 Realm。
每轮写 r0/r2/r4 的 tag/payload；boxed/receiver/shape 容量均0，`0x8` 是空向量指针，不可用于冲突推断。
补采作业另记录 slot名、received cell/payload 和 unpack（下述补采结果）。

这六组实测 ops 字段读范围与 scalar 写范围的低12位**没有重叠**。8f/dfff 的 wide locals 与 bank
虽同为页0，但前者在 typed 数值内环不访问，而且快的8f也如此。Realm低位610也没有区分两版本。
因此“global cell 与 bank 的4K假相关解释42.7%”没有得到支持。这里只排除了捕获路径中的这些直接流；
没有排除机器栈真实 spill、其他执行阶段、代码前端、TLB、更多流的 set 竞争。

iMac L1D32KiB、line64；虚拟低位可以筛查页内重合，**不足以证明真实 L2/LLC set conflict**，
更不能从 malloc 地址推出物理页着色。捕获的192-byte WideLoopFrame 在 caller native 栈上，
不是正在执行的 closure 的 RBP/spill 地址；本轮没有原镜像热 spill 的完整实址。需要同镜像因子实验
或可归属的硬件事件才能把剩余硬件假说升级为根因；不能用本轮带日志 wall 补这个缺口。

补采 `20261021T075245Z-t056-architecture-supplement` 对三版本的 whole unpack 各记录92,000次
wide返回、8个program；未捕获 typed执行（这些短回调没有循环）。第一次30-op/18-register回调：

| 镜像 | wide register backing | ops | frame Vec | constants / 首cache payload | 首结果 StringData / text |
| --- | --- | --- | --- | --- | --- |
| base | `0x7f9b60757510` | `0x7f9b60757280` | `0x6000015ded00` | `0x6000007dc400` / `0x7f9b60711100` | `0x600000def730` / `0x600002bcd7e0` |
| 8f | `0x7ff4abf1cf90` | `0x7ff4abf55ef0` | `0x6000002b4600` | `0x6000010b4400` / `0x7ff4abf11140` | `0x600001a86dd0` / `0x600003ca4b00` |
| dfff | `0x7fa8b6f1cf90` | `0x7fa8b6f55ef0` | `0x600002ce8600` | `0x600003ee8400` / `0x7fa8b6f11140` | `0x6000034dadd0` / `0x6000012f8b00` |

8f/dfff这些首样本的页内余数相同，不能区分其计数差；其余program和动态字符串地址仍在变。
日志在回调结束后，不包含 regexp native 的机器栈、所有对象/UTF数据的同时活跃访存，且后续92,000次日志
本身会干扰分配；它满足实址清单，不能证明 unpack 内环无冲突。首结果 matcher-view=0，非活跃地址。
补采也证实 global 的r5来自slot0 `broadGlobalOne`：received cell 与realm cell地址相同（例如dfff
`0x60000358caf0`，Value在+8），不是另建的全局快照。所有完整原值见
[补采记录](../diagnostics/t056-architecture/supplement-evidence.json)；不同诊断镜像不混做地址对照。

## 代码布局：外层定址没有固定内部控制流

两个 canonical 镜像 SHA：8f=`b0b84489a3f6fd673b06dc4bdc71c8f176387a231ca19fe1da5ea48ca4ee0fd5`；
dfff=`3466c37a62f2fb6d764d0123212749ff810138ca05fcad336ce1afa05bdb1e3f`。
与 base 的六段数值热路径分别检查指令、原字节和 rel/RIP relocation：Lt32条/140B、Exit12/51、
Add32/141、Update17/81、Dispatch13/48、Backedge4/22，均 address-normalized equivalent。
这是有限数值路径审计，不是整个函数等价或每个输入动态执行相同的证明。

8f/dfff 的执行入口都是 `0x100007d90`，但内部 Lt/Add/Update/Exit 若干块移动3–12字节；
dispatch table 从 `0x10000b23c` 到 `0x10000b230`，numeric binary table 从b2c0到b2b4；
Update读取的−1.0常量从 `0x1004b0708` 到 `0x10049c648`。
同入口、同 normalized bytes 并没有固定 fetch/解码边界、分支目标位置或常量数据流。
目前不能量化42.7%中有多少来自前端，不能断言是某个 Intel erratum。

[layout_variants.py](../../tools/benchmark/layout_variants.py) 的 K=6 是固定
`(0,16,64,240,512,1008)` NOP atom 位移，审计 natural text inventory 的统一平移和 body 不变。
它覆盖一部分代码地址敏感性；不重新随机化 LLVM 内联图、函数内部块布局，也没有显式改变
malloc size class、free list、活对象历史或热数据相对地址。链接变化可能间接移动只读段，
所以也不应声称它“严格只变指令缓存”。当前面板缺乏**受控的数据布局维度**。

## 必须纳入的代码布局先例

| 记录 | 实测事实 | 推论边界 |
| --- | --- | --- |
| [T050](T050-vm-layout-causality.md#results-and-conclusion) | 只移动WideLoopFrame热循环、固定其余共享函数：global1.129073、local1.082748、branch1.083914；对应instructions1.000041/1.000045/1.000033；global A/A0.999176 | 代码位置本身可复制这类回退的方向/大部分幅度；不是已证明某个cache/predictor或当前43%的根因 |
| [T058 attempt1](T058-attempt1-incomplete.md) | top_level_function_call natural1.4316；五对1.046/1.018/1.491/1.526/1.432；K=6六布局为1.4316/1.0354/1.0113/1.0112/1.0413/1.0189 | 约43%的natural事件也可能在其余5布局消退；单natural不能证明持续，仍须panel |
| [T054](T054-execution-layout-regions.md#sole-neutrality-screen-and-rejection) | 所有4205函数原页余数保留，却有recursive_call_tree1.067917、instructions1.000096，sentinel lane1.007214超1.005而拒绝 | entry/residue pinning不等于中性；五对有波动，不能把6.8%当成已隔离原因或普遍幅度 |

当前实址清单也要准确区分：8f/dfff的scalar/realm/wide-local页内模式相同，**typed ops是600对900**，
不是所有数据低12位都相同；unpack首组主要流低位相同。它们没有给出bank冲突的因果证据。
结合canonical热字节等价而内部块/跳表/常量位置变化，先验证代码侧是更有证据的路径，
而非断言所有数据因素已排除。K=6只平移现有body，不能检验重新编译后的内联图隔离程度。

## 成熟引擎的具体对照

| 引擎 | 已有设计事实 | 对本项目的含义 |
| --- | --- | --- |
| QuickJS-NG（本仓库 pin f7830186） | `JSStackFrame` 保存 arg/var/stack cursor；`JS_CallInternal` 用一次 alloca 容纳需复制的参数、locals、operand stack，var_buf/stack_buf 为相对片段；参数有时借用 argv；逃逸变量经 JSVarRef 保活 | 一个 activation 的 Value 数据有共同物理所有者，非每类一个 malloc；不是所有对象都进栈。[源码](https://github.com/quickjs-ng/quickjs/blob/f7830186043e4488f2998759d60a514faf07cbc9/quickjs.c#L17630) |
| V8 Ignition | `RegisterLocation` 用 interpreted frame pointer 加寄存器偏移；x64 entry 根据 bytecode frame size 分配机器栈寄存器文件；handler 从 dispatch table 读取并尾派发 | 热寄存器窗口及 dispatch ABI 明确，冷对象类型不会改变 register-window 格式；仍有独立 bytecode/feedback/heap。[register/dispatch](https://github.com/v8/v8/blob/main/src/interpreter/interpreter-assembler.cc)、[entry](https://github.com/v8/v8/blob/main/src/builtins/x64/builtins-x64.cc) |
| Lua 5.4 | lua_State 拥有 stack/top；CallInfo 标记函数/帧顶，VM 用 base+register-index；栈增长统一修正调用帧和 open upvalue 指针 | 堆上连续栈也能有稳定相对布局；增长必须处理逃逸引用，不能保留失效 slice。[状态](https://www.lua.org/source/5.4/lstate.h.html)、[执行](https://www.lua.org/source/5.4/lvm.c.html)、[增长](https://www.lua.org/source/5.4/ldo.c.html) |
| LuaJIT v2.1 | `resizestack` 统一 realloc lua_State.stack 并重定位 base/top/open-upvalue | 单一 VM 栈与显式增长协议配套，而非单纯改容器。[源码](https://github.com/LuaJIT/LuaJIT/blob/v2.1/src/lj_state.c) |
| SpiderMonkey | InterpreterFrame 的 slots 紧随 frame，base越过nfixed；InterpreterStack 用 LifoAlloc（默认chunk4KiB）分配 frame+values，按 mark 释放 | 帧头/局部/operand 有共同生命周期和物理邻接；允许分块，不要求无限连续大Vec。[定义](https://github.com/mozilla/gecko-dev/blob/master/js/src/vm/Stack.h)、[分配](https://github.com/mozilla/gecko-dev/blob/master/js/src/vm/Stack.cpp) |

这些引擎仍会受 heap/code/cache 布局影响；借鉴的是可解释的栈所有权和内核 ABI，不能照搬 C alloca、
汇编或声称成熟引擎免疫布局。本项目可先用 safe Rust Box/片段完成，不以 unsafe、NaN boxing 或 GC 为前提。

## 目标A：从冷类型和fat-LTO图中隔离小热内核（主线）

当前Cargo release是fat LTO、codegen-units=16；LLVM仍能跨原有crate/CGU看见大部分运行时代码。
内核需要显式、窄的Rust ABI：ops base、零基scalar/boxed窗口、pc/limit、固定宽度冷服务handle，
返回类型化的Exit/Deopt/ServiceRequest。对象/Realm/CallEnv实体与其enum匹配不进入内核定义。
FrameId在入口/退出/safepoint解析一次；不能每op重新查帧、算动态base或调trait object。
数字和wide opcode/dispatch保持分开。结构性冷墙在真正离开kernel时调用，不能每个数值op都过桥。

| 可评估选项（未实施） | 成本/限制 | 非计时字节与placement审计必须证明 |
| --- | --- | --- |
| kernel独立crate或非LTO代码生成边界，runtime只依赖窄kernel IR/退出请求 | 新crate/API迁移和代码复制；fat LTO下仅换crate**不构成隔离**。需后续构建配方资格，不能暗改当前recipe。kernel不能反向依赖runtime，禁止FFI | 冷ObjectData/CallEnv字段大小等中性变体不改变kernel指令/内部块/常量引用/栈spill；依赖和跨边界call清单；若LTO重新合并则失败 |
| 同crate小非泛型kernel + `#[inline(never)]`服务边界；调用者处理退出后再进入 | call/return、入口保存恢复、失去特化；`inline(never)`不阻止全部IPO/clone/常量传播，不能声称绝对隔离 | 解析实际cold call目标、clone/合并符号、热闭包大小和spill；中性冷代码变动前后逐块比对，禁止给局部热点持续堆标注代替分层 |
| kernel专用codegen单元/对象与可审计布局清单 | Rust模块不是CGU保证，fat LTO会再合并；可能需独立编译单元及额外构建维护 | pinned toolchain实际CGU/object归属、最终链接后的kernel完整性；审计dispatch header、handler入口/尾部、每跳表项目标和RIP常量相对位置，不能仅看函数entry余数 |
| 构建层LLVM loop/block对齐策略（先确认pinned backend支持） | NOP/代码体积、I-cache压力和别处回退；跨平台/版本支持不同；不能试一堆偏移选赢家 | 对齐前后全部内核块布局/填充、跳距/分支跨界、只读表位置和总text增长；保留机器可读目标映射，单一前瞻策略再跑完整诊断 |
| 独立threaded handler函数/dispatch表 | Rust fn-pointer call可能无法尾调用、增加间接分支/栈增长、失去跨op优化；不使用computed-goto/unsafe/FFI | 核实每handler控制流、尾调用是否真实、最深native栈及每op call/return数量、寄存器保存；audit不同冷类型构建的body/表布局；若每op累计Rust栈则拒绝 |

优先评审“小非泛型kernel+真实冷服务边界”，必要时升级编译边界；上述不是五次获准性能尝试。
先冻结一种一般化结构、内存/指令/复制预算与中性变化审计，再实现。字节稳定不能证明更快；
但若中性冷类型变化仍重排热块，便没有达到此架构目标。最终性能仍需完整K=6/原计划，不靠pin一个好地址。

## 目标B：ExecutionContext数据所有权（M2，不是global43%的已知修复）

沿用[ExecutionContext/FrameStore/FrameId](../../docs/design/interpreter-core-v2.md)，context拥有分段固定backing；
帧头、local/register、operand窗口有共同生命周期。LinkedCode对小反馈采用真正共同backing或索引，
不是装着多个Vec就叫共址；immutable code共享，realm/context反馈隔离。稳定global handle仍指向
唯一Upvalue，保留删除/accessor/eval/with失效协议；不重引入每调用name-keyed快照。

数据packet可减少独立热流和malloc邻域依赖，但现有数值内环仅ops+scalar，不能预期它修复当前global。
不先实现HotExecutionPacket、不为它选择赢的offset；是否需要它由代码×数据受控试验决定。
一个arena也可能让多个页内同offset流发生假相关，不能承诺硬件免疫。
成本包括段/反馈内存、decoded拷贝、入口搬运、增长/reentry/回收协议。
按[frame activation约定](../../docs/design/frame-activation.md)只借零基窗口一次，避免T021逐访问base成本；
增长/suspend/deopt前归还slice，cell身份不变，getter/proxy/native回调前释放可变/RefCell借用。
捕获、递归、generator/async、throw、eval/with、cross-realm都要验证；numeric packet也不能替代unpack的
native regexp/string和短回调路径。M2不以unsafe、NaN boxing、GC或新依赖为先决条件。

## unpack-code：指令增加的已知归属与边界

lead 的 1.0016→1.0119 是各自对 base 的计数；本轮作业
`20261021T071529Z-t056-architecture-instruction-attribution` 用同一8f对照、每项3对，
只解释 instructions，不解释其 instrumented/ablation wall：

| dfff 变化的独立消融（其余保持8f） | unpack instructions / 8f |
| --- | ---: |
| 完整 dfff 原镜像 | 1.010213 |
| header/flag/count 布局及 class-field，撤回两处 lookup_for_write | 1.012125 |
| getter adapter only | 1.000193 |
| write-index admission + 两处 mutating lookup | 1.009803 |

因此 adapter 不是主要新增计数来源；header 与写索引路径均能触发增量，但**不是可相加成本分解**。
少建 transient hash index 会增加后续线性查找，packed flags/checked count 也改变生成代码；本轮没有
逐次 lookup 比较计数，不能把整个1%精确记到某条源语句。此前 native malloc 记录说明 StringData
仍为72/Rc88、class96；新版本减少464个索引、928次分配，约1.38MB累计request，却没有减少总指令。

原镜像[静态图](../diagnostics/t056-architecture/native-graph-evidence.json)确实变了：8f 的
`call_native_family` frame0x3f8、`call_regexp_native`0x478、`replace_matches`0x288；
dfff 前者0x308，出现 `native_regexp_prototype_replace`0x2f8，后两者已内联而无独立符号。
未改的 symbol_replace 源码也受全程序内联图影响；wide store_owned 及主 frame0x388 基本不变。
新增的 `Vec<usize>::from_iter` outlined helper位于 substitution_length，但 unpack 的 functional-replacer 路径
未证实执行该分支，不能把它当作已证明热点。静态行数/消失符号均不是动态执行成本。

数值包原型不能承诺消除 unpack 的回退：其 native regexp/string、短回调、对象与 matcher 数据仍在
另一条路径。把 unpack 当架构范围的独立检验；若它仍敏感，须补 native-boundary/call窗口和代码图归属，
不能宣布 scalar 变稳就完成了整个任务。

## 里程碑与测量顺序

1. lead运行未改dfff vs f58的诊断K=6 mini-panel（6布局×5对、双方A/A、9固定用例）；
   [README](../diagnostics/t056-code-layout/README.md)预注册excess消退/持续/不明确规则和≤30分钟预算。
2. 消退≥5/6只允许lead**考虑**花最后attempt，不能生成acceptance。持续则不花，先运行已准备的
   代码×数据2×2：同一编译body的code0/16B平移 × MallocNanoZone0/1，global/local/empty/unpack，
   每格双方A/A。所有因子在结果前固定，不能换“更显著”的zone/pad。混合/噪声结果也不支持直接花attempt。
3. 若需要代码内核修改，先冻结主线A的窄ABI与编译隔离原型；非计时中性冷类型变化审计通过，
   再做完整正确性和前瞻性能单元。若2×2发现数据交互，再细化M2所有权原型，不从natural直接推演packet。
4. 后续方法可资格化code×data panel及A/A交互噪声带；不修改已冻结v6、K=6偏移、旧结果和attempt预算。
   N/2N差分不保证数据地址一样；需要记录实际活跃流。2×2同code格内只改环境，不重编译；code格之间
   是relocation-normalized相同body的两镜像，不能称整个文件SHA一样，也不能据此断言纯I-cache原因。

## 本轮核验

本次修订基于b847ed4d，生产树仍等于dfff；`./scripts/check.sh`全部通过（含5170 subset）。
非计时qualification382s：12个布局镜像均NG255/输出相同、body/address审计通过；zone资格18s：8格均NG255/输出相同。
真实collector用fake process/counters完成mini1080与cross960单元、路由/AA/环境/夜间和超时守卫；
[版本化报告](../diagnostics/t056-code-layout/validation.json)明确不是timing。文档context、staged fast及whitespace核验通过。
无生产代码、依赖、公开API、policy或attempt变化；架构选项未实现，两个lead-only timing任务均未提交。
