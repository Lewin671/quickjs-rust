# T056：global_read 源码分组双向消融

- Status: diagnostic；无验收、无 attempt 消耗。
- Implementation: 生产代码仍为 dfff3bb6；所有消融只写非计时队列的源码archive副本。
- Verified at: 802aa51a 生产树；分组全量重建 f84..dfff 文件通过，八份镜像构建/审计结果见下。
- Evidence: [hunk、镜像和lead协议](../diagnostics/t056-bisect/README.md)；[原PMU/浮点诊断](T056-pmu-attribution.md)。
- Unresolved: PMU仍受阻；本轮未提交计时，不能确定哪个源码分组触发global回退。
- Next action: lead执行≤25min五block双向消融；先看addition≥1.30且removal≤1.05，再调查命中分组的机制。最后一次attempt不动。

## 穷尽构建输入，避免混合 hunk

`git diff f84fec3c dfff3bb6`共21个文件。其中9个在Rust recipe构建输入范围内，共41个hunk。
8f7d97c7→f84fec3c仅两个任务文档变化，crates/tools/scripts/.cargo/Cargo清单无差异。
[完整hunks.json](../diagnostics/t056-bisect/hunks.json)保留每个hunk的原header、全文、分组、SHA，
以及其余12个docs/tasks/诊断脚本文件被排除的原因。没有遗漏的生产代码或order改动。

| 文件（省略runtime src前缀） | hunk数 | 分组 |
| --- | ---: | --- |
| qjs-cli/hot-functions-x86_64.order | 11 | d |
| bytecode/typed_loop/execute.rs | 2 | c |
| property/layout.rs | 2 | b |
| property/layout/tests.rs | 1 | b，测试不进入release |
| value/object.rs | 19 | a=17，b=2 |
| value/object/class_fields.rs | 3 | a |
| value/object/flags.rs | 1 | a，含cfg(test)测试 |
| value/object/storage.rs | 1 | b |
| value/object/tests.rs | 1 | a，测试不进入release |

a=22hunks：ObjectData的u32 index count、单字节flags/字段布局、全部flag读写与class-field计数；
b=6：large-shape失败写probe不升温index，lookup_for_write及storage/object的全部调用点；
c=2：ObjectRef导入、typed getter的object-only非内联helper与adapter；d=11：完整order更新。
测试随对应组改变，避免不存在的flags类型或lookup方法留在消融测试树中。

`partition.py`逐个校验旧上下文和行数；空分组逐字节等于8f文件，全分组逐字节等于dfff文件。
因此不是用“整个object.rs归header”的粗分类：两个lookup_for_write hunk明确属于b。
每个plus只含一组；每个minus在8f上应用其余三组，与dfff移除指定组等价。

## 八份构建、canonical与字节审计

非计时构建队列：`20261021T112221Z-t056-source-bisect-build` **done0 / 764s**。
每份独立源码archive、统一canonical agents/fat-LTO/16CGU/abort/align16 recipe，显式host target；
同一target cache顺序编译后立即复制qjs，未改生产checkout或canonical缓存，未二进制patch/重签名/挑padding。
十个待比较镜像为plus_a..d、minus_a..d及canonical8f/dfff；另外保留canonical f58作共同reference。
源码文件hash、recipe、toolchain、build-log hash、binary hash、检查结果写入images.json。
plus_a/b/c分别逐字节复现071529Z既有header/write_index/adapter镜像（独立archive路径与测试hunk也被重建），
独立确认本轮canonical recipe可复现。
全部11镜像输出校验通过，全部六条重复路径归一化字节等价；不把构建或smoke耗时当性能证据。
Lt32条/140B、Exit12/51、Add32/141、Increment18/86、Dispatch13/48、Backedge4/22。
Increment常量为+1.0，避免复用旧Decrement审计。完整hash与PC/relocation见images.json；以下短hash只便于阅读。

| 镜像 | SHA-256前12位 | dispatch头地址 | Add payload store相对执行器offset |
| --- | --- | --- | --- |
| aligned | `b0b84489a3f6` | `0x100007e16` | `0x29ca` |
| candidate | `3466c37a62f2` | `0x100007e16` | `0x29be` |
| base | `b0dfc65d0c02` | `0x10025be16` | `0x2a66` |
| plus_a | `23ffd979b63c` | `0x100004e06` | `0x29be` |
| plus_b | `23461f0f5b8c` | `0x100006e16` | `0x29ca` |
| plus_c | `9774d685a236` | `0x100004e16` | `0x29ca` |
| plus_d | `674baf84d332` | `0x100007e26` | `0x29ca` |
| minus_a | `ccf60fb9f0fc` | `0x100006e26` | `0x29ca` |
| minus_b | `84b5623cf17d` | `0x100004e16` | `0x29be` |
| minus_c | `c3e1aafb8837` | `0x100007e16` | `0x29be` |
| minus_d | `08ec0adbd48d` | `0x100007e06` | `0x29be` |

所有11镜像检查global/local在N=23,068,672与2N的输出一致，另查对象写入、索引删除、
preventExtensions/freeze与枚举语义。诊断镜像只用于这次分组定位，不代替完整Test262正确性验收。
审计选择实际finite-number路径：Lt、true Exit、Add、Increment、Dispatch、Backedge，
相对8f比较指令字节，仅归零已验证branch/RIP displacement；数字常量referent8也必须一致。
保留每条原始/normalized字节、PC相对偏移、relocation目标及handler/header位置；不以函数字节总hash代替路径检查。
这是global/local共用的重复路径，不覆盖入口、cold helper或退出清理；归一化等价不代表PC布局等价。
若某消融改变了路径，必须明确报告，不把该项装作纯placement实验。

## lead 预注册判据与时限

五个paired block，每case各一份f58共同reference、十个镜像、8f/dfff各自独立inode的同字节A/A副本：
13roles×3cases×5blocks=195单元，global/local各N/2N、empty一次，共325个计数执行加195个launch preflight。
角色/case顺序轮转并反转，N/2N交替；同一active路径与环境，完整保留所有样本，不重跑/删除异常。
记录wall、retired instructions；不使用已被cycle-trust否定的rusage cycles。

**分组trigger：其加入8f后global_read/f58中位数≥1.30，且从dfff移除后≤1.05。**
先确认同作业8f/f58≤1.05、dfff/f58≥1.30；global/local的8f与dfff A/A中位数在[1/1.03,1.03]，
95% CI在[1/1.10,1.10]，指令中位数在[1/1.01,1.01]，否则只报告threshold match、qualified trigger为空。
CI按五个配对block bootstrap封存并完整报告，不临时放宽阈值；empty A/A报告但不作为精度gate。
没有组、多个组、单向命中、AA失败都保留；多个命中表示交互/非唯一归属，不擅自挑一个。
这些是源码组合trigger，尚不是硬件根因；所有image共享五个f58 block reference，不是十组独立实验。

以实际stack probe的1440单元/1049.23s换算，本作业195单元约142.08s；3倍余量+120s约546.25s。
collection1200s、body1380s、runner起点1490s终止守卫、watchdog1500s（含quieting，不含排队）；
仅停止本作业后代。host本地03–05禁测、03前15min保护、入场需30min；原始文件不完整时不判断。
lead命令与render/watch见README。owner只提交non-timing构建，无计时作业、无预算/冻结policy改动。

## 命中后应查什么（目前均为假说）

| 组 | 首查路径与候选机制 | 不能直接声称 |
| --- | --- | --- |
| a | ObjectData120→112、Rc请求136→128；对象初始化/flags代码、启动分配历史与scalar bank/program/ops邻接；同时核验下游LLVM hot/cold PC关系 | 不是每轮多次flag/property访问：现有global typed op只有数值操作；zone/stack阴性结果不能直接排除所有heap相互作用 |
| b | probe失败时不建index，改变启动layout/index分配与首次binding解析；成功写probe admission及下游LLVM布局 | global steady-state没有逐轮shape lookup，不能把lookup减少/增多直接乘N解释43% |
| c | object getter独立helper、array/non-object adapter帧/尾调用、LTO跨函数影响和相关call/branch PC | 这条global typed program没有GetNamed op；只有触发后才深入证明是入口seed、代码图还是其他间接影响 |
| d | linker函数顺序、相对代码/常量位置、caller/callee与预测器/前端关联；检查hotpath字节与位置同时变化情况 | K=6/16B统一平移已经不足以解释该事件，但完整order改变不等于统一平移；不能直接命名DSB/alias硬件原因 |

下一步只解释lead结果命中的组及其交互，不先实施生产修复或继续无界layout猜测。

## 本提交核验

802aa51a生产树未变；非计时作业done0、三个既有ablation镜像逐字节复现。
`validate.py`通过：真实collector用模拟process走完520次调用、五个block、13角色，
验证trigger/无trigger/单向/多组、AA/anchor、不完整、错输出/超时/扰动/hash/deadline拒绝，
以及同active路径、分离inode、清env、DST/25min守卫；模拟数据绝非计时结果。
本地`./scripts/check.sh`通过（含5170/5170 Test262 subset），context/file-size/whitespace通过。
staged fast与pre-push全门禁随本提交执行。无runtime/parser/lexer语义变化，fast无额外语义slice。
没有生产改动、依赖、公开API或冻结计划变化；没有owner计时提交，最后一次attempt仍未消耗。
