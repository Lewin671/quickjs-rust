# T056：d130 数据与启动状态探针（已完成，历史协议）

job `20261021T040650Z-t056-m1a-data-probe` 已 done0：540/540 单元、401秒、无 DISTURBED。
结果与限制见[后续记录](../../archive/T056-m1-stageA-aligned-scratch.md)。以下命令保留用于审计，不重跑本探针。

此协议用于诊断 `d130c77b` / canonical `66ffb46f` 的 local_read wall 回退。
[诊断记录](../../archive/T056-m1-stageA-data-diagnosis.md)拥有结论、CPU/机器码证据和前置 job；
[protocol.py](protocol.py)拥有固定 case、条件、比较和预算，配置哈希见[probe-config.json](probe-config.json)。
不修改冻结 unit、阈值或次数；不提交最后一次 K=6。原 probe A 不完整，不补 pair；旧 B guard 不变。

## 条件和解释

固定三个 case，均为15个完整 block，串行轮换/反转顺序：

| Case | 每 block 条件数 | 使用的镜像 |
| --- | ---: | --- |
| local_read | 16 | canonical 两侧各5条件；两侧各1个诊断镜像，各3条件 |
| heterogeneous_property_read | 10 | 只有原 canonical 两侧各5条件 |
| string-unpack-code | 10 | 只有原 canonical 两侧各5条件 |

Canonical 条件：自然设置、child-only MallocNanoZone=0、=1、128字节 env cookie、byte-identical A/A。
每次切换都清空全部因子键再设置，TZ/LC_ALL固定；所有条件经同一个字面路径执行并预热。
同一镜像的因子角色使用同一 inode；A/A使用独立的相同字节副本。

Bank条件只跑 local_read。每个实现增加一个诊断镜像，在同一机器码中选择`page00`或`page64`，
再加page00 A/A。两种状态都分配8192字节、4096对齐的 backing，借用其中4096字节的寄存器窗口，
起点低12位分别为0和64；实际地址已通过非计时trace核对。两个env值都为6字节。
Box、切片和`try_into`均为安全Rust，没有FFI、unsafe或全局可变状态。

**bank64/bank00才是同代码、同分配大小的地址因子。** bank00/canonical同时包含诊断源码、
header、backing和代码位置变化，是偏差对照；不能把它的改善当成原候选已修好。
Bank因子不会用于另外两个case，避免把诊断镜像的代码变化混入它们的比较。

前置已完成：d130完整exact、绑定86-case预门、原生T061 triage；同镜像bank两种状态的smoke、
NG255和typed-loop tests；bank64/bank00指令增量约1.000005/1.000031。配置逐项绑定这些receipt和SHA。
所有前置必须done0、无DISTURBED；旧484和不完整probe A不能满足新协议。
本探针已完成。`render.py`只打印脚本文本，不执行、不联网、不提交job。

## 预算

A在1800秒完成9×9×15=1215个case/条件/block单元；新探针为(16+10+10)×15=540。
按同吞吐为800秒，再乘1.3补偿内部case占比并留120秒校准/分析，约1160秒。
不再沿用旧15–25分钟估计。采集硬上限1260秒，主体1380秒；本地watchdog把含runner quieting的
running时长限制为1500秒（排队等待不计）。主体也读取runner开始时间，提前留10秒终止余量。
系统ZoneInfo夜间03:00–05:00限制保留；窗口不足直接退出。

可能触顶而不完整；这时保留全部raw，不补block、不删pair、不选择性重跑。
不能减到15个block以下。必须使用下面的watchdog，不能只用裸`wait`替代。
watchdog仅终止这个命名job的pane及其子进程树，不改变服务或CPU设置。

## lead 执行

```bash
REPO=/Users/qingyingliu/Code/co-work-space/worktrees/quickjs-rust/t056-m1-stage-a
PROBE="$REPO/tasks/diagnostics/t056-data-probe"
RUNNER=/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh
PROTO=$(git -C "$REPO" rev-parse HEAD)
T056_DATA_REPO="$REPO" PYTHONDONTWRITEBYTECODE=1 python3 "$PROBE/validate.py"
"$RUNNER" status
JOB=$("$RUNNER" run --repo "$REPO" --ref "$PROTO" --with d130c77b7b2e216a3f8e6cf68892ee28dd632c22 --with f58c696e8379261ed726f0cea1051ed6d34f431a --timing --name t056-m1a-data-probe -- bash -c "$(python3 "$PROBE/render.py")")
printf '%s\n' "$JOB"
T056_BENCH_RUNNER="$RUNNER" python3 "$PROBE/watch.py" "$JOB"
"$RUNNER" log "$JOB"
"$RUNNER" fetch "$JOB"
```

审阅request、complete、全部raw/preflights和effects.json；完整性必须是3个case、15 blocks、
16/10/10条件、540单元。先看canonical local C/B是否复现和两类A/A是否稳定，
再看各实现内zone/env、bank64/bank00及C/B交互。区间是未经多重调整的探索性95% bootstrap，
没有advance/acceptance判定。其他两个case若出现相同因子响应，只支持共同原因方向，不能证明相同根因。
若within-image指令数也改变，先解释执行路径，不能直接归因地址延迟。

## 重现静态审计

```bash
cd "$REPO"
PYTHONPATH="$REPO" python3 "$PROBE/audit.py" --base /path/to/b0dfc65d-qjs --candidate /path/to/66ffb46f-qjs --output-dir target/t056-local-byte-audit
```

它检查物理SHA、抽取实际指令字节、验证并归一化rel8/rel32及RIP位移，核对数字常量，
保留每条指令、原始/归一化字节、地址和32字节分支边界。不会启动被审计进程。
`validate.py`另用fake process/counters跑完整collector，验证环境重置、同inode、A/A、raw和540单元；
模拟产物在临时目录删除，校验报告写入target，不是计时证据。
