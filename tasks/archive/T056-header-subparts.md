# T056：ObjectData 子消融与完整循环访存归属

- Status: diagnostic；无验收、无 attempt 消耗。
- Implementation: 生产树仍为dfff3bb6；仅archive诊断镜像、回放器、证据与lead-only协议。
- Verified at: 3df764e4生产树；114227Z lead数据与canonical完整反汇编/实际操作数快照核对。
- Evidence: [完整记录与协议](../diagnostics/t056-header/README.md)；[上轮源码消融](T056-source-bisect.md)。
- Unresolved: a组内部触发因素与硬件原因未定；未提交本轮计时，PMU仍受阻。
- Next action: lead运行≤15min三因素全矩阵；按完整结果判断触发单项/交互，随后评审修复，最后一次attempt不动。

## lead 结果：a 是唯一双向匹配，原精度结论保留

`20261021T114227Z-t056-source-bisect` done0/148.15s，五block/195单元输出全一致。
global C/B：8f .992032、dfff1.425716；plus_a1.557997、plus_b .982613、plus_c .996283、plus_d1.000629；
minus_a .997476、minus_b1.426922、minus_c1.428416、minus_d1.568039，指令比约1。
local .921836–.972720；empty指令在包括8f的全部镜像为约1.106–1.109，+10%属于更早stage A初始化。

lead据此把a选为诊断触发组。原自动评估仍为inconclusive：AA_eligible=false、threshold_matches=['a']、
qualified_triggers=[]，具体为dfff global A/A CI下界.752862低于1/1.10。
未删除样本、改规则或把该结果升级为验收；[原评估、全部effect与文件SHA](../diagnostics/t056-header/lead-result.json)保留。

## 子因素：count、flags、order，没有新增的cold payload迁移

- count：ObjectData index_property_count的usize→u32、constructor checked转换，以及class-field累计类型；其他计数器不变。
- flags：五个Cell<bool>合为ObjectFlags(Cell<u8>)，全部get/set、导入/module/default初始化随之改变；hot typed-array/module-namespace bool仍独立。
- order：ObjectData声明与同名constructor字段的顺序变换，含两个hot bool移到prototype之前、prototype/symbol相对位置调整；保留该cell所选的类型。

ObjectColdData字段内容完全未变，cold OnceCell本来就在末尾；“cold/hot moves”是order轴的一部分，
不是另一项把payload搬出header的改造。Rust repr(Rust)允许编译器重排，必须读取实际offset，不能拿源码行序当物理布局。
[parts.py/parts.json](../diagnostics/t056-header/README.md)封存精确源文件哈希：六个proper非空子集，
加8f空集合、既有full_a三因素全集，覆盖2^3格；另保留dfff与f58 reference。
空集合与8f源文件逐字节一致，全集与上轮a组源文件逐字节一致。测试专用112B guard只在全集使用，partial保留原120B上界。

供计时的镜像严格使用原agents/fat-LTO/16CGU/abort/align16 recipe，未patch/重签名/挑padding。
布局导出另加-Zprint-type-sizes，每个variant使用独立fresh target；这些额外编译产物绝不用于计时。
初始作业115238Z的镜像构建、输出、全部数值热路径审计已成功；布局解析在flags阶段漏掉qjs_runtime前缀，
并发现共享target会复用较早source变体的runtime类型信息，故整个作业done1，旧flags布局不可信。
初始producer由build-initial.py保留。后续120731Z qualification验证原recipe镜像hash、独立重编译全部布局，
显式要求runtime crate新发出无qjs_runtime前缀的ObjectData信息，避免只拿CLI侧缓存布局当证据。

布局资格`20261021T120731Z-t056-header-layout-qualification` done0/583s；
初始作业`20261021T115238Z-t056-header-subparts-build` done1/733s，原recipe镜像全部有效，失败只在后续布局导出。
[完整ObjectData字段offset](../diagnostics/t056-header/layouts-summary.json)及原始类型块已封存。

| 源码因素 | ObjectData bytes | RcInner bytes | index_count offset/width | packed flags offset/width |
| --- | ---: | ---: | --- | --- |
| aligned | 120 | 136 | 72/8 | 独立bool |
| count | 120 | 136 | 104/4 | 独立bool |
| flags | 120 | 136 | 72/8 | 112/1 |
| order | 120 | 136 | 72/8 | 独立bool |
| count_flags | 112 | 128 | 104/4 | 108/1 |
| count_order | 120 | 136 | 104/4 | 独立bool |
| flags_order | 120 | 136 | 72/8 | 112/1 |
| full_a | 112 | 128 | 104/4 | 108/1 |

单缩count或单pack flags均为120B；两者同时才为112B。这里只是布局事实，尚无本轮wall结果。
三个轴是独立源码变换，**生成布局/LLVM代码并非物理独立**：count从8B变4B时，repr(Rust)把它从offset72移到104，
即使没有选order轴也会发生。order轴表示源码声明/初始化顺序，不能声称固定其他物理offset。
所有格properties RefCell范围为0..55，full_a/count_flags的flags在108，与其不相邻；index_count104..107与flags108为分离访问范围。
同8B机器word不构成已验证的部分重叠。RcInner列是实际类型size，不是malloc可用size/实际cache-line对齐的测量。

## 完整循环：152条，而不是150条

使用canonical f58/8f/dfff整个反汇编、原始Mach-O jump table/常量、104647Z的实际bank/ops/register快照，
从dispatch(pc0)独立回放到rbx+1后的下一次dispatch(pc0)。没有拼接旧六条path来预设后继。
所有条件分支由实际操作数计算，间接跳转由真实表字节计算；未知opcode、地址、控制输入或非有限float立即拒绝。
三版本、两case、四检查点共24次回放一致；这是有实际输入约束的静态回放，**不是硬件branch trace**。

四个op的handler计94条（Lt32+Exit12+Add32+Increment18）；四次完整dispatch共52条；
op流耗尽还执行cmp/jae **2条**；backedge reset/bound/increment/branch共4条，合计152。
local增加Add32+dispatch13，因此197。所有路径无call、无RSP调整；residency bound高于2N，
i/sum在2N内均为远小于2^53的整数，Number tag保持，正常Exit只在最终条件失败时发生。
已有完整logical trace为单次TLRUN/Ran、无deopt；结合状态归纳覆盖整个正常重复区间。

| global 指令核算 | f58 | 8f | dfff |
| --- | ---: | ---: | ---: |
| 模型N增量 | 3,506,438,144 | 相同 | 相同 |
| 实测五block增量中位数 | 3,508,095,726 | 3,507,918,670 | 3,508,752,317 |
| 模型/实测增量 | 99.95275% | 99.95779% | 99.93405% |
| 已知重复区间/完整N进程指令 | 99.06715% | 99.02267% | 99.00323% |

这是instruction覆盖估计，不是cycle/time份额；残差含区间外工作、OS/counter口径和噪声，不能将其全算作某个函数。
固定startup的大量指令在N/2N中消去，steady loop占绝大多数增量，优先解释其吞吐变化有依据；
但等指令数本身不能排除少数很慢的区间外指令、调度等wall因素，更不能直接命名machine-clear/forwarding事件。

## 每轮全部55次显式数据访存：没有ObjectData或RefCell borrow flag

[mechanism.json](../diagnostics/t056-header/mechanism.json)逐条保存152/197条指令、真实bytes、全部branch后继、EA/宽度/区域。
image地址使用未加ASLR slide的Mach-O VM地址；bank/ops/program/stack为实际捕获进程地址。
区域互不重叠，任何访问落到五类之外立即报错，不把未识别地址默认为“非ObjectData”。

| global每轮访问 | 次数×宽度 | 基址/offset |
| --- | --- | --- |
| scalar tag/bool读 | 7×1B | bank+r2/r3/r4/r5的16B槽起点、r0 tag与bool |
| scalar f64读 | 5×8B | bank+16*r+8，r2/r3、r4/r5、Increment r2 |
| scalar tag/bool写 | 5×1B | bank+{0,1,32,64,65} |
| scalar payload写 | 3×8B | bank+{8,40,72} |
| opcode/operand读 | 16×1B、1×4B | ops的4个16B record，4B为Exit ip |
| program头读 | 9×8B | ops指针+8读4次、len+16读5次 |
| scratch指针spill读 | 2×8B | [RBP−0x30]；读出后本路径不解引用该指针 |
| 只读jump table/常量 | 6×4B、1×8B | dispatch4次、Binary2次、Increment +1.0 |

所有写都在scalar bank；global ObjectData/flags/index count/prototype、RefCell borrow flag、
realm/global binding cell都没有逐轮地址。源码execute借用TypedLoopScratch.registers，global_reads在入口seed；
实际ops是Lt/Exit/Add/Increment，完整路径没有隐蔽的getter/exotic检查或native调用。
因此“每轮写RefCell借用字、再读相邻packed flag，引发部分重叠forwarding失败”**不符合本条路径**。
对全部load/store EA做交集检查：没有不同起点或宽度的实际重叠；同一bank tag/payload的依赖均是同址同宽。
上述是architecturally taken路径，不含I-fetch、未退休/错误路径投机访存。
这不排除disjoint地址的假依赖、投机与代码前端等其他硬件机制；这些仍需子消融与进一步归属，不能混称为flags重叠。

## 布局规则与修复建议的适用范围

作为通用设计约束：高频可变计数/借用状态使用自然对齐、明确宽度的独立存储；hot flags保留独立字节，
避免热路径对覆盖相邻字段的宽字进行RMW。若实测必须固定offset，用审计过的HotHeader显式布局；
cold flags可在cold payload或独立尾部单元打包。cache-line隔离需按真实访问和allocation成本决定，不能默认每对象补一整条cache line。
若未来确认该类部分重叠，修复是分离实际重叠的hot状态与flags存储、统一相关读写宽度，复核release指令、offset与语义；本轮不实现。

[Intel优化手册§3.6.4.1](https://cdrdv2-public.intel.com/671488/248966-Software-Optimization-Manual-V1-048.pdf)
说明forwarding有大小/对齐约束，具体限制依微架构变化；**仅同word/cache line或地址低位相同不等于真实部分重叠**。
当前152条路径没有该违规访问，所以不能把这条原则直接当成本事件的既定修复。先由lead子因素结果决定后续方向。

## 单个≤15min lead作业

global+local、N/2N、五shared paired block；9个镜像vsf58，再加8f/full_a/dfff各一份独立inode A/A，共13角色、130单元、260 measured+130preflight。
固定active路径与env，旋转/反转顺序，不删异常、不重试。以114227Z真实148.15s/195单元，校正全internal成本后约111.11s；3倍+120s约453.34s。
collect720s、body840s、runner起点890s守卫、watchdog900s，含quieting不含排队；保留03–05禁测及03前15min保护，入场需20min。
单项≥1.30且补集≤1.05为双向match，全部7个子集的median/CI与最小observed bad subsets均报告；
A/A与anchor未通过只保留未qualified结果，不换阈值。无单项但pair/triple变化时报告交互候选，不自动追加实验。
没有owner计时提交、生产修复、依赖/公开API或冻结计划变化；最后一次attempt保留。

## 本提交核验

原recipe六镜像构建、十镜像N/2N输出与数值路径字节审计通过；八份布局均由独立fresh target核验。
子消融镜像的目标输出校验不构成完整conformance验收；本地全门禁验证的是未改变的生产树。
`validate.py`通过：真实source/receipt/image/audit hashes，24次完整回放，未知opcode/地址/NaN拒绝，
以及fake-process真实collector的390次调用、单项/组合/AA/anchor/incomplete及输出/扰动/超时/hash/deadline/owner拒绝。
这些模拟输出不是计时证据。`./scripts/check.sh`通过（含5170/5170 Test262 subset），context/file-size/whitespace通过。
staged fast/pre-push门禁随本提交执行；生产树与3df764e4相同，fast无新增runtime/parser/lexer语义slice。
