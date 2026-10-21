# T056：152条路径的PC与地址分类、ops相位确认

- Status: diagnostic；无验收/attempt消耗，未提交计时。
- Implementation: 生产树仍为dfff3bb6；原镜像unsigned终止捕获与限定numeric程序的诊断副本。
- Verified at: 7492fa7c生产树；123327Z header结果、124843Z/130515Z入口捕获、125859Z探针输出/字节核验。
- Evidence: [分镜像完整指令/访存/地址及协议](../diagnostics/t056-dispatch-address/README.md)；[header分解](T056-header-subparts.md)。
- Unresolved: code-PC映射与ops主相位同时变化，尚无唯一硬件根因；PMU仍受阻。
- Next action: lead运行≤15min ops地址确认，先通过sham/同相位/AA中性门，再解释相位效应；随后评审通用修复。最后一次attempt保留。

## 已有计时：count+flags交互合格，尚非硬件归属

`20261021T123327Z-t056-header-subparts` done0，AA_eligible=true，minimal bad subset=[count_flags]。
global C/B：8f1.006479、dfff1.436013、full_a1.563363、count1.000792、flags .998029、order1.001753、
count_flags1.569505、count_order1.002008、flags_order .996488；local .936844–.975632。
[原评估与SHA](../diagnostics/t056-dispatch-address/header-result.json)完整封存，无样本删除、规则改变或预算消耗。
112B是交互后的布局事实；f58同为112B且快、zone0未消除dfff回退，因此不将它直接解释为malloc尺寸根因。

## 三镜像直接比较：相同152条序列，不是相同PC/原始machine code

count与flags的全部相对PC及32/64B位置一致，8f只是整体+0x3000，低12位代码位置也相同。
count_flags与dfff共享另一个相对PC映射。相对count的152个动态位置中：96个不变、12个−3B、24个−6B、20个−12B。
所以“热块byte-identical”只能指归一化branch/RIP displacement后的opcode序列；56个位置实际移动，部分原始displacement也变了。
count_flags的完整真实输入回放仍是152条（local197），**不是每条PC都相对函数不变**。

| 镜像 | 入口 / dispatch头VM地址 | Binary表LEA相对PC | 公共payload store相对PC |
| --- | --- | --- | --- |
| count | 0x100004d90 / 0x100004e16 | 0x1909 | 0x29ca |
| flags | 同count | 0x1909 | 0x29ca |
| count_flags | 0x100004d80 / 0x100004e06 | 0x1903 | 0x29be |
| 8f | 0x100007d90 / 0x100007e16 | 0x1909 | 0x29ca |
| dfff | 0x100007d90 / 0x100007e16 | 0x1903 | 0x29be |

[loops/](../diagnostics/t056-dispatch-address/README.md)每镜像/case逐条给出顺序、PC、relative PC、raw bytes、实际后继，
以及每条指令32/64B窗口起止、cross/end标志和实际ASLR process PC；全部55次global数据访存也逐条列EA/宽度/类别。
数据区域为bank+off、ops+off、program+8/+16、RBP−48（当前原始RSP+0x160）以及RIP派生jump table/FP literal。
这些是完整taken路径的实参驱动回放，不是硬件branch trace；未知地址/op/control拒绝，不把未知访问默认归入某一类。

## RIP目标：全部地址明确，但没有一个literal相位统一区分快慢

下表为未加ASLR slide的VM地址；捕获证实slide为4096倍数，故low12也是实际process low12。
每组首个实际process地址及全部间接表entry EA在对应loop记录中；不能把不同进程的ASLR高位直接当布局因素。

| 镜像 | TypedOp jump table / low12 | BinaryOp table / low12 | +1.0 FP literal / low12 |
| --- | --- | --- | --- |
| count | 0x10000823c / 23c | 0x1000082c0 / 2c0 | 0x1004ac730 / 730 |
| flags | 0x10000823c / 23c | 0x1000082c0 / 2c0 | 0x10049cd48 / d48 |
| count_flags | 0x100008220 / 220 | 0x1000082a4 / 2a4 | 0x10049c180 / 180 |
| 8f | 0x10000b23c / 23c | 0x10000b2c0 / 2c0 | 0x1004b0750 / 750 |
| dfff | 0x10000b230 / 230 | 0x10000b2b4 / 2b4 | 0x10049c698 / 698 |

FP值均+1.0、8B自然对齐且不跨64B边界。count/flags的PC相同却FP目标不同且都快；
两个慢镜像的literal low12也不同，不能挑一个constant位置称作已证实的根因。

## 真实入口heap：ops主相位变化，其余热基址模式一致

`20261021T124843Z-t056-dispatch-heap-capture` done0/2s：5镜像×global/local×N/2N×3，共60份。
只修改unsigned副本的dispatch入口6B和未执行handler区的终止桥；headers/长度/映射section及其他全部字节保持原样。
桥保存原RSP后才push，通过write syscall导出，无logger/native调用帧/引擎分配，不重签名。
同一active路径/argv/env；这些是当前捕获进程实址，**不是之前计时进程的回溯地址**。

| low12或相对位置（每镜像每case6份） | count / flags / 8f | count_flags / dfff |
| --- | --- | --- |
| global bank / program | 000 / 000 | 相同 |
| global ops | 600 | 900 |
| local bank / program | 000 / 600 | 相同 |
| local ops | 600 | 900 |
| heap constant_registers（global/local） | 420 / 360 | 相同 |
| received r5 cell位置/值 | 原始指针见各capture，Number1.0 | 同一low12模式/Number1.0 |
| global program−bank | +0x3000 | 相同 |

每镜像首个global/N/rep0进程的实际指针（并非旧timing进程）：

| 镜像 | bank | program | ops | heap constants | received r5 cell |
| --- | --- | --- | --- | --- | --- |
| count | `0x7ff55b884000` | `0x7ff55b887000` | `0x600001a13600` | `0x600001010420` | `0x600003a34af0` |
| flags | `0x7fa6db084000` | `0x7fa6db087000` | `0x600000673600` | `0x600000c70420` | `0x600002654af0` |
| count_flags | `0x7f90ef884000` | `0x7f90ef887000` | `0x600002d95900` | `0x600002798420` | `0x600000dbcaf0` |
| aligned | `0x7f858f084000` | `0x7f858f087000` | `0x600002e0f600` | `0x60000240c420` | `0x600000e28af0` |
| candidate | `0x7f9186084000` | `0x7f9186087000` | `0x600003c69900` | `0x600003664420` | `0x600001c40af0` |

实际bank、ops、program、heap constants及received-cell RcInner/cell/payload地址全部逐样本记录；
received slot通过Bytecode cell mask、program.local_slots与WideLoopFrame.upvalues索引联接，不靠按名称猜测。
heap constant-register Vec与compiled +1.0 literal是两种对象，记录中明确分开；二者都不因“constants”一词而混同。

每轮heap读只有ops、program头和scalar bank；ops地址的主分布确实随交互变化。
received cell和heap constants只在入口seed。backedge/budget为立即数cmp/lea/ja，无heap interrupt/safepoint polling。
program.deferred_global_publication为true，但sloppy_global_writes.len为0：wrapper退出后最多调用一次空zip发布，
不是每轮隐藏publication；完整152条中没有该flag/cell/ObjectData访问。

**已有观察不能唯一选定一个原因。** 相对PC族是确定性的快慢分类；ops600/900又是同时变化的地址分类，
但local在900仍快。后续sham捕获还出现flags原始ops的两个离群相位（900/180），说明malloc相位也非普遍不变量。
全部保留，不用地址筛耗时。不能从这两个耦合因素中凭相关性宣布一个单独PC、cache set、forwarding或machine-clear事件。

## 已准备的因果确认：代码完全固定，只改ops位置

`20261021T125859Z-t056-ops-address-qualification` done0/28s：count/flags/count_flags各三份副本：
sham保留原ops、phase600/phase900把相同ops bytes放到同一个4K匿名RW页的两个位置；所有副本都复制两份数据。
在原函数最后的spill/首次jump处重定向入口，在正常Exit false分支重定向cleanup；
入口重放原spill、保留GPR/RFLAGS、临时reserve32B，RBP及所有原spill EA不变；正常退出前恢复原program.ops指针与RSP。
页面限定一页/子进程，随CLI退出回收。没有新native调用帧，原function headers/长度与所有非patch字节保持原样。
**这次探针会改入口尾部字节，不应称为完整prologue原封不动**；早期producer docstring的该措辞已澄清，执行payload SHA与current AST除docstring一致。

每条重复指令的raw bytes、PC和RIP targets均与其parent逐字节相同，循环内不加branch/load/store；
变的只有program.ops的值与实际被读的ops buffer地址。parent库仍保留原buffer，cleanup先恢复后再进入原退出/释放路径。
桥放在该fixture未执行的Vm monomorphization区域，所有patched镜像仅允许这两个已核验的normal numeric case，不能作为通用engine二进制。
十四镜像global/local N/2N输出全部正确，探针不替代完整conformance验收。

`20261021T130515Z-t056-ops-address-capture` done0/4s：108份同方法入口捕获，所有强制600/900均匹配，bank仍000，原RBP相对heap模式不变。
sham的RSP也同样少32B；flags-sham十二份中十份600、一份900、一份180，原样记录，无重跑或过滤。

## 通用规则/候选修复与15min判据

通用目标是让只读execution plan/control数据和numeric hot code，均不依赖ObjectData表示与普通对象分配历史。
当前可评审的候选是显式immutable execution view（ops slice/len等），与mutable scalar bank及interior-mutable caches分离；
ObjectData专用guard/adapter经稳定ABI的cold helper隔离，使其字段编码不再任意移动numeric blocks。
这保留slot/upvalue cell模型，不是per-call名字快照，也不按benchmark源码挑路径。

若相位确认支持数据地址作用，再评审独立的plan/op存储所有权/arena与code/data交互；
**不能直接把600硬编码成“快地址”或恢复120B当作通用修复**。若数据相位无效，优先解释确定的PC族/冷代码内联影响。
所有候选均需另行冻结实现计划及广泛布局/语义门禁，本轮不改生产，不花最后attempt。

[lead协议](../diagnostics/t056-dispatch-address/README.md)：17roles（14镜像、f58、两个sham A/A），global/local、五paired block，
170单元、340 measured+170preflight。以114227Z实测148.15s/195单元，校正全internal成本并缩放170单元约145.30s，3x+120s<720s；
collect720/body840/runner890/watch900（含quieting，不含排队），03–05禁测及03前15min保护，需20min入场余量。

先要求sham/original、同相位relocation/sham在global/local均中性（median与95%CI在±5%带），
两个sham A/A合格、count/flags快与count_flags/dfff慢anchor重现。随后看慢镜像600恢复≤1.05且600/900 upper CI<1/1.10，
以及至少一个快镜像900再现≥1.30且900/600 lower CI>1.10；满足支持ops相位因果作用。
只出现一侧为source-dependent敏感性；中性失败为setup/domain confound；中性但无效则所测相位解释不成立。
所有原始行、异常和CI保留，不选择最佳地址、不删pair、不重试，无PMU事件归属或验收结论。

## 本提交核验

7492fa7c生产树未变；十四镜像输出qualification通过、60原始与108探针入口捕获通过，强制相位全部匹配。
`validate.py`通过：真实receipt/image/capture seal、全部patch与patch之外字节、所有重复PC/byte/window、真实输入完整回放，
以及fake-process实际collector的510次调用和causal/source-dependent/null/confounded/AA路由、输出/扰动/hash/超时/deadline/owner拒绝。
模拟数据不是计时结果。`./scripts/check.sh`通过（5170/5170 subset）；context/file-size/whitespace通过。
staged fast与pre-push门禁随提交执行；无新增runtime/parser/lexer语义slice、依赖、公开API或冻结计划变化。
