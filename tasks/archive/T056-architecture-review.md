# T056：解释器热状态与布局稳健性评审

- Status: architecture review；未实现目标架构，未消耗验收 attempt。
- Implementation: 本轮只增加文档和隔离诊断生产器；生产代码仍是 dfff3bb6。
- Verified at: b1ecbaf6 的生产树；base=f58c696e、aligned=8f7d97c7、allocation=dfff3bb6 的独立 iMac 诊断见下表。
- Evidence: [机器可读地址与字节审计](../diagnostics/t056-architecture/address-evidence.json)、[复现及 lead 探针](../diagnostics/t056-architecture/README.md)。
- Unresolved: global_read 的 42.7% wall 回退尚无硬件因果归属；不把低 12 位相同当作证明。
- Next action: lead 取得独立模型设计评审，再决定是否运行已准备的稳健性诊断、冻结架构原型；本轮不实施、不提交 timing/K=6。

## 判断与阶段决定

**应暂停最后一次 K=6，先验证热状态布局与执行内核的结构边界。**
继续调整某个对象头、标量数组偏移、order file 或 inline 标注，不能成为 T056 的架构答案。
这些改动已经使回退在 local、unpack、global 之间迁移。它们说明现有测量对布局敏感，
但还不能证明所有回退具有同一个 malloc/4K-alias 根因。

最新 lead 链：`20261021T060648Z-t056-m1a-allocation-correctness` 完整 exact 42,672/42,672、
NG255/subset5170/full pass；`20261021T062918Z-t056-m1a-allocation-pregate86` 86/86；
`20261021T063231Z-t056-m1a-allocation-triage` 为 **likely loss**、diagnostic、attempts=0。
相对 f58：global wall **1.4271 [1.4197,1.4376]**、instructions **1.0002**；
local .948、branch .949、heterogeneous .898；unpack wall **1.0381 [1.0247,1.0688]**、
instructions **1.0119**。这些是 lead 的原镜像结果，不是本轮带日志镜像的 wall。

Stage A 仍是未集成的属性布局实验，不能因已投入修补而晋级。建议把“布局所有权和稳健性”列为
继续该阶段的前置条件，与已有 [M2 设计](../../docs/design/interpreter-core-v2.md) 的 frame/context
迁移合并评审；原型按新的前瞻计划实施，不在旧计划下无限调参。
如果 lead 要先收窄 A，只能在测量前明确修订范围和兼容关系；不能删除失败控制、重置 attempt，
也不能把旧 K=6 解释为新范围的通过。有效 attempt 1/2 已用，A+B 共剩一次。

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

## 目标：一个执行上下文的布局所有权，以及小而固定的内核

沿用 [ExecutionContext/FrameStore/FrameId](../../docs/design/interpreter-core-v2.md)，由 context
拥有分段、固定 backing 的解释器栈/arena。一个帧的 header、local/register window、operand window
在同一分配段内；typed 区域不再从每个 program 的独立 Box/Vec 池重新拼出热状态。
段大小和布局来自执行数据的类型、容量和对齐约束，不来自某个 benchmark 名称/字节码模板。
FrameId 带 generation；扩大、reentry、suspend、deopt 在显式 safepoint 归还所有 slice，再借新窗口。
捕获值仍指向唯一 Upvalue；不能使 `Vec` 内裸地址跨增长存活，也不能复制权威 cell。

热内核只接收 ops base、零基寄存器窗口、pc/limit 和冷服务 handle。FrameId 在入口/退出/
safepoint 解一次；**不得每 op 重新查帧、加动态 base、建立 trait object 或重做整窗清零**。
数字和 wide 的 opcode/dispatch 仍分开；不把 generic object/native slow path 内联进数值大 match。
对象和 realm 通过固定宽度 handle/typed view 过边界，让 ObjectData/CallEnv 的无关字段变化不改热
参数形状和寄存器活跃区间。用字节/spill 审计检验这个边界，不能以新增 inline 标注替代它。

LinkedCode 拥有已解码 ops 和小型 per-site feedback：可用同一个有界 chunk 的固定区/索引，
而不是一个装着许多 Vec/Rc 的 struct 就叫“共址”。immutable code 可共享；realm相关反馈必须按
realm/context 隔离，避免共享代码把不同 realm 的 shape/cell 混起来。大/稀疏 cache 放冷 side table。
全局在链接/入口解析为稳定 cell handle，保留 global deletion、accessor、eval/with 的现有失效协议。
循环语义允许时才 seed/hoist；不能为了稳定地址缓存可被 getter/回调修改的值。

这会减少独立热 backing、使帧内偏移与 unrelated ObjectData 大小无关，避免每次 malloc 邻域决定
register/operand 相对位置。它**不是全部硬件布局不变的保证**：外部 JS heap、机器栈、只读常量和代码仍在。
仅把两个4KiB数组并排会让同偏移落在同一页内余数，甚至加重4K假相关；“一个 arena”本身不够。

最小原型可先做一个有界 **HotExecutionPacket**：固定 header、decoded-op 小窗、所需 boxed/operand
小窗，最后放完整256×16 scalar bank。由声明的最大同时触及 footprint 决定 admission，
使小区域真正活跃的 op字段/register前缀落在同一页内的**不重叠区间**；未用 scalar tail 可跨页。
完整bank保留现有索引不变量；safe Rust 借不同字段。大程序走一般分段窗口，不按 global/local 名称特判。
这不是再挑一个赢的 bank offset：物理布局由字段/边界推导并在扰动前冻结，不能搜索最佳 padding。
原型约束若不能覆盖常用区域，应改整体分段布局而非永久把引擎限制为小循环。

成本：每 context/code 的 decoded copy 与反馈内存、可能从4KiB增至约8KiB的小包、入口搬运、
段增长和回收复杂度、cache维护；code/shared metadata 与 per-context写反馈的重复。
T021 的 window-base 每访问成本与 T023 arena 的失败已有历史，必须重用
[frame activation 的 driver/窗口约定](../../docs/design/frame-activation.md)，不能复活被测慢的路径。
getter/proxy/native回调前释放可变借用；不能跨可执行JS持有 RefCell borrow。
closure、递归、generator/async suspend、throw/unwind、eval/with、cross-realm 都需要迁移验证。

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

## 里程碑、反证条件与测量变化

1. **设计评审和现状诊断**（本轮）。保留原失败；lead 独立评审小内核/共同布局是否覆盖实际热流。
   已准备15-block、A/A的≤25分钟探针，仅测8f与dfff，不实现新架构，不申请 acceptance。
2. **前瞻冻结最小原型**。先固定 HotABI/布局约束、一般化 admission、instruction/复制/内存预算。
   同一窗口只解一次 base。先做真实 footprint/活期分析，不靠 allocator lucky address。
3. **同镜像分配扰动反证**。固定64个无关活块，尺寸128/144/1536；两套 A/A、代码/输出身份、
   空程序和 canonical-vs-diagnostic 偏差控制全保留。先在当前实现看到数据地址变化及可检测敏感性，
   才能用该因子证明原型削弱了它。若旧版本对该因子不敏感、AA噪声过大或因子没有移动相关流，
   结论是 probe不足，不能“原型修复”。原型需新配置/资格，不能替换旧记录镜像冒充同一试验。
4. **扩大到 native/短回调/general call**。让 unpack/local/global 同时对固定扰动稳健；保留
   branch/heterogeneous 控制和真实大程序。对冷类型尺寸等中性重构重新构建，审计内核 body/spill；
   这检验同镜像数据扰动不能覆盖的 inlining/codegen 敏感性。
5. **再决定 A 的最后 K=6**。仅在先验预算、正确性和稳健性均足够时继续；不能用诊断胜负替代86、
   exact和完整selection/formal。仍不通过则关闭/重塑阶段，保留结构学习而非继续调位置。

未来布局方法应考虑 **code×data 因子**：相同代码镜像内改变无关分配历史/大小，跨代码位置保持因子
对称，报告交互项和A/A；布局 panel 也要记录入口/2N关键地址及活跃范围。
受控因子不等于穷尽 heap随机化，更不应寻找最佳分配种子。现有 K=6/v6 冻结证据不改；
只有经独立资格与噪声带校准后，才能把数据维度纳入后续政策。N/2N的差分取消固定工作量，
不保证两次进程具有相同数据布局；本次global两N低位相同，不能推广到所有输入。

探针的具体命令、镜像SHA、预算、夜间排除窗口与不完整结果处理由
[diagnostic README](../diagnostics/t056-architecture/README.md) 唯一维护。本轮未提交任何 timing 任务。

## 本轮核验

b1ecbaf6的生产树未变；本轮 `./scripts/check.sh` 全部通过（含5170个subset）。
最终文档的 `check-context.py`、diff whitespace检查通过；诊断byte audit重新执行通过。
collector的真实流程用fake process/counters完成15×6×12单元；验证报告已提交在diagnostic目录。
无新依赖、公开API、运行时代码或policy变更。上面的架构仍是提案，风险与反证条件并未因工具通过而消失。
