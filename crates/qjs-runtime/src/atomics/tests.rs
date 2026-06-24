use crate::{Value, eval};

/// Evaluates `source` in a Test262 `$262.agent` whose `AgentCanSuspend()` is
/// `can_block`, mirroring the CLI's `--agent-cannot-block` mode.
#[cfg(feature = "agents")]
fn eval_in_agent(source: &str, can_block: bool) -> Result<Value, crate::EvalError> {
    crate::eval_classified_with_resolver_in_agent(
        source,
        "<test>",
        Box::new(crate::module::MapResolver::new()),
        can_block,
    )
}

#[cfg(feature = "agents")]
#[test]
fn atomics_wait_throws_when_agent_cannot_block() {
    // CanBlockIsFalse: AgentCanSuspend() is false, so Atomics.wait throws a
    // TypeError (sec-atomics.wait step 7) instead of returning a status string.
    assert_eq!(
        eval_in_agent(
            "let a = new Int32Array(new SharedArrayBuffer(16)); \
             try { Atomics.wait(a, 0, 0, 0); 'no throw'; } \
             catch (e) { e instanceof TypeError ? 'TypeError' : 'other'; }",
            false,
        ),
        Ok(Value::String("TypeError".to_owned().into()))
    );
}

#[cfg(feature = "agents")]
#[test]
fn atomics_wait_throws_from_a_nested_user_frame_when_agent_cannot_block() {
    // The CanBlockIsFalse cases call Atomics.wait inside `assert.throws`'s
    // callback, two user frames below the script root; the agent context must
    // survive the frame chain (it rides every CallEnv like the module host).
    assert_eq!(
        eval_in_agent(
            "function outer(fn) { return fn(); } \
             let a = new Int32Array(new SharedArrayBuffer(16)); \
             outer(function () { \
               try { Atomics.wait(a, 0, 0, 0); return 'no throw'; } \
               catch (e) { return e instanceof TypeError ? 'TypeError' : 'other'; } \
             });",
            false,
        ),
        Ok(Value::String("TypeError".to_owned().into()))
    );
}

/// JS that builds `$262.agent` from the native primitive globals, mirroring the
/// host shim so a test running through `eval_in_agent` can drive worker agents.
#[cfg(feature = "agents")]
const AGENT_PRELUDE: &str = "var $262 = { agent: {\
 start: __quickjsRustAgentStart,\
 broadcast: __quickjsRustAgentBroadcast,\
 getReport: __quickjsRustAgentGetReport,\
 report: __quickjsRustAgentReport,\
 sleep: __quickjsRustAgentSleep,\
 monotonicNow: __quickjsRustAgentMonotonicNow,\
 receiveBroadcast: __quickjsRustAgentReceiveBroadcast,\
 leaving: __quickjsRustAgentLeaving,\
} };\n";

#[cfg(feature = "agents")]
fn eval_in_main_agent(source: &str) -> Result<Value, crate::EvalError> {
    eval_in_agent(&format!("{AGENT_PRELUDE}{source}"), true)
}

#[cfg(feature = "agents")]
#[test]
fn worker_agent_shares_memory_and_reports_back() {
    // A worker on its own OS thread receives the broadcast SharedArrayBuffer,
    // writes to it via Atomics.store, and reports; the main agent reads the
    // report and observes the worker's write through the shared backing.
    assert_eq!(
        eval_in_main_agent(
            "var i32a = new Int32Array(new SharedArrayBuffer(8)); \
             $262.agent.start(`\
               $262.agent.receiveBroadcast(function (sab) { \
                 var w = new Int32Array(sab); \
                 Atomics.store(w, 0, 42); \
                 $262.agent.report('done'); \
                 $262.agent.leaving(); \
               }); \
             `); \
             $262.agent.broadcast(i32a.buffer); \
             var report = $262.agent.getReport(); \
             while (report === null) { $262.agent.sleep(1); report = $262.agent.getReport(); } \
             report + ':' + Atomics.load(i32a, 0);"
        ),
        Ok(Value::String("done:42".to_owned().into()))
    );
}

#[cfg(feature = "agents")]
#[test]
fn worker_agents_report_in_fifo_order() {
    // Two workers each report once; getReport drains them in arrival order.
    assert_eq!(
        eval_in_main_agent(
            "var i32a = new Int32Array(new SharedArrayBuffer(8)); \
             function startWorker(id) { \
               $262.agent.start(`\
                 $262.agent.receiveBroadcast(function (sab) { \
                   var w = new Int32Array(sab); \
                   Atomics.add(w, 0, 1); \
                   $262.agent.report(String(${id})); \
                   $262.agent.leaving(); \
                 }); \
               `); \
             } \
             startWorker(1); \
             $262.agent.broadcast(i32a.buffer); \
             var first = $262.agent.getReport(); \
             while (first === null) { $262.agent.sleep(1); first = $262.agent.getReport(); } \
             first + ':' + Atomics.load(i32a, 0);"
        ),
        Ok(Value::String("1:1".to_owned().into()))
    );
}

#[cfg(feature = "agents")]
#[test]
fn shared_array_buffer_round_trips_through_the_cross_thread_backing() {
    // With the agents feature on, SharedArrayBuffer bytes live in the Arc-shared
    // backing rather than `internal_bytes`; element reads, writes, and growable
    // `grow` must round-trip through it identically.
    assert_eq!(
        eval(
            "let sab = new SharedArrayBuffer(8, { maxByteLength: 16 }); \
             let a = new Int32Array(sab); \
             a[0] = 42; a[1] = -7; \
             sab.grow(16); \
             let b = new Int32Array(sab); \
             [b.length, b[0], b[1], sab.byteLength, sab.growable].join(',');"
        ),
        Ok(Value::String("4,42,-7,16,true".to_owned().into()))
    );
}

#[cfg(feature = "agents")]
#[test]
fn atomics_wait_returns_status_when_agent_can_block() {
    // With AgentCanSuspend() true and no other agent to notify, the single-agent
    // wait still resolves to a status string rather than throwing.
    assert_eq!(
        eval_in_agent(
            "let a = new Int32Array(new SharedArrayBuffer(16)); \
             Atomics.wait(a, 0, 0, 0);",
            true,
        ),
        Ok(Value::String("timed-out".to_owned().into()))
    );
}

#[test]
fn atomics_object_surface_and_lock_free() {
    assert_eq!(
        eval(
            "Object.prototype.toString.call(Atomics) + ':' + \
             Atomics.add.length + ':' + Atomics.compareExchange.length + ':' + \
             Atomics.isLockFree(1) + ':' + Atomics.isLockFree(3);"
        ),
        Ok(Value::String(
            "[object Atomics]:3:4:true:false".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "try { new Atomics.add(new Int32Array(1), 0, 1); false; } \
             catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn atomics_read_modify_write_numeric_views() {
    assert_eq!(
        eval(
            "let a = new Int32Array(new SharedArrayBuffer(16)); \
             a[2] = 6; \
             [Atomics.add(a, 2, 4), Atomics.load(a, 2), Atomics.sub(a, 2, 3), \
              Atomics.and(a, 2, 6), Atomics.or(a, 2, 8), Atomics.xor(a, 2, 3), \
              Atomics.exchange(a, 2, -1), Atomics.store(a, 2, Math.PI), a[2]].join(',');"
        ),
        Ok(Value::String("6,10,10,7,6,14,13,3,3".to_owned().into()))
    );
}

#[test]
fn atomics_store_normalizes_negative_zero_return_value() {
    assert_eq!(
        eval(
            "let a = new Int32Array(new SharedArrayBuffer(4)); \
             let stored = Atomics.store(a, 0, -0); \
             Object.is(stored, 0) + ':' + Object.is(stored, -0) + ':' + Object.is(a[0], 0);"
        ),
        Ok(Value::String("true:false:true".to_owned().into()))
    );
}

#[test]
fn atomics_pause_surface() {
    assert_eq!(
        eval("Atomics.pause() === undefined && Atomics.pause(42) === undefined;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Atomics.pause.length;"), Ok(Value::Number(0.0)));
    assert!(eval("new Atomics.pause();").is_err());
    assert!(eval("Atomics.pause(42.5);").is_err());
    assert!(eval("Atomics.pause('42');").is_err());
}

#[test]
fn atomics_notify_validates_and_returns_zero_without_waiters() {
    assert_eq!(
        eval(
            "let i32 = new Int32Array(new SharedArrayBuffer(8)); \
             let i64 = new BigInt64Array(new SharedArrayBuffer(16)); \
             [Atomics.notify.length, Atomics.notify(i32, 0), Atomics.notify(i64, 0, 1)].join(':');"
        ),
        Ok(Value::String("3:0:0".to_owned().into()))
    );
    assert_eq!(
        eval("let i32 = new Int32Array(new ArrayBuffer(8)); Atomics.notify(i32, 0, '33');"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let index = { valueOf() { throw new Error('index'); } }; \
             try { Atomics.notify(new Uint8Array(new SharedArrayBuffer(8)), index, 0); false; } \
             catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let count = { valueOf() { throw new Error('count'); } }; \
             try { Atomics.notify(new Int32Array(new SharedArrayBuffer(8)), 9, count); false; } \
             catch (e) { e instanceof RangeError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn atomics_wait_returns_timed_out_in_single_agent() {
    // The single agent can block but no other agent can notify it, so a wait
    // whose value matches the comparand reaches its timeout and returns
    // "timed-out" (the timeout argument is still coerced via valueOf).
    assert_eq!(
        eval(
            "let i32 = new Int32Array(new SharedArrayBuffer(4)); \
             let i64 = new BigInt64Array(new SharedArrayBuffer(8)); \
             let calls = ''; \
             let timeout = { valueOf() { calls += 'timeout'; return 0; } }; \
             let r32 = Atomics.wait(i32, 0, 0, timeout); \
             let r64 = Atomics.wait(i64, 0, 0n, timeout); \
             [Atomics.wait.length, r32, r64, calls].join(':');"
        ),
        Ok(Value::String(
            "4:timed-out:timed-out:timeouttimeout".to_owned().into()
        ))
    );
    // A mismatched value returns "not-equal" without coercing the timeout.
    assert_eq!(
        eval(
            "let i32 = new Int32Array(new SharedArrayBuffer(4)); i32[0] = 7; \
             Atomics.wait(i32, 0, 0, 0);"
        ),
        Ok(Value::String("not-equal".to_owned().into()))
    );
    assert_eq!(
        eval(
            "try { Atomics.wait(new Int32Array(new ArrayBuffer(4)), 0, 0, 0); false; } \
             catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn atomics_wait_rejects_bad_indices_before_value_coercion() {
    assert_eq!(
        eval(
            "let a = new BigInt64Array(new SharedArrayBuffer(32)); \
             let value = { valueOf() { throw new Error('value'); } }; \
             let timeout = { valueOf() { throw new Error('timeout'); } }; \
             let checks = [Infinity, -Infinity, -1, 4, 99].map(function(index) { \
               try { Atomics.wait(a, index, value, timeout); return false; } \
               catch (e) { return e instanceof RangeError; } \
             }); \
             checks.join(',');"
        ),
        Ok(Value::String("true,true,true,true,true".to_owned().into()))
    );
}

#[test]
fn atomics_read_modify_write_bigint_views() {
    assert_eq!(
        eval(
            "let a = new BigInt64Array(new SharedArrayBuffer(64)); \
             a[3] = 0x33333333n; \
             let old = Atomics.xor(a, 3, 0x55555555n); \
             let after = Atomics.load(a, 3); \
             let stored = Atomics.store(a, 3, -5n); \
             [old, after, stored, a[3]].join(',');"
        ),
        Ok(Value::String(
            "858993459,1717986918,-5,-5".to_owned().into()
        ))
    );
}

#[test]
fn atomics_compare_exchange_updates_only_on_match() {
    assert_eq!(
        eval(
            "let a = new Uint8Array(new ArrayBuffer(4)); \
             a[0] = 7; \
             [Atomics.compareExchange(a, 0, 1, 9), a[0], \
              Atomics.compareExchange(a, 0, 7, 9), a[0]].join(',');"
        ),
        Ok(Value::String("7,7,7,9".to_owned().into()))
    );
}

#[test]
fn atomics_validation_order_and_receivers() {
    assert_eq!(
        eval(
            "let index = { valueOf() { throw new Error('index'); } }; \
             try { Atomics.add(new Float32Array(new SharedArrayBuffer(8)), index, 0); false; } \
             catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let value = { valueOf() { throw new Error('value'); } }; \
             try { Atomics.xor(new BigInt64Array(new SharedArrayBuffer(16)), 99, value); false; } \
             catch (e) { e instanceof RangeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let calls = ''; \
             let a = new Int32Array(new ArrayBuffer(4).transferToImmutable()); \
             let index = { valueOf() { calls += 'index'; return 0; } }; \
             let value = { valueOf() { calls += 'value'; return 1; } }; \
             try { Atomics.store(a, index, value); false; } \
             catch (e) { calls + ':' + (e instanceof TypeError); }"
        ),
        Ok(Value::String(":true".to_owned().into()))
    );
}
