/*
 * Generic-path sentinels for quickjs-rust and QuickJS-NG.
 *
 * `broad-micro.js` measures how well the engine's specializing tiers recognize
 * a shape: its callees are statically resolvable and its receivers are fixed,
 * so a partial evaluator can fold the measured operation away entirely. That
 * makes it a specializer coverage suite, not a measure of the ordinary
 * interpreter.
 *
 * These cases keep the same host contract but deny the specializer the static
 * facts it needs, using ordinary dynamism rather than artificial barriers:
 * callee identity and receiver identity are selected from runtime-built
 * tables, receivers carry more than one storage shape, and the recursive case
 * returns its result through a binary call tree whose value cannot be proven
 * without induction. Every case still yields a closed-form checksum so the
 * host can verify the work actually ran.
 *
 * If a future optimization genuinely makes ordinary calls or ordinary property
 * access fast, these cases must move. That is the point: they are the holdout
 * the specializer cannot answer by pattern-matching a benchmark.
 *
 * The host measures process wall time. This program contains no timer.
 */

function fail(message) {
    throw new Error("generic sentinel workload: " + message);
}

function parseIterations(text) {
    var value = Number(text);
    if (!Number.isFinite(value) || value < 0 || Math.floor(value) !== value) {
        fail("iterations must be a non-negative integer");
    }
    return value;
}

function result(operations, checksum) {
    return { operations: operations, checksum: checksum };
}

/*
 * Recursion depth for the call-tree case. Every top-level invocation performs
 * 2^(DEPTH + 1) - 1 calls, so the case measures call setup and teardown rather
 * than the loop around it.
 */
var CALL_TREE_DEPTH = 6;
var CALL_TREE_CALLS_PER_ITERATION = 127;

/*
 * Returns `value + 1` for every depth, but only by way of a binary recursion
 * whose result flows through both branches. Establishing the identity requires
 * induction over the tree, which is why no partial evaluator can replace the
 * calls with their result.
 */
function callTree(depth, value) {
    if (depth <= 0) {
        return value + 1;
    }
    var left = callTree(depth - 1, value);
    var right = callTree(depth - 1, value);
    return left + right - (value + 1);
}

function runRecursiveCallTree(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += callTree(CALL_TREE_DEPTH, i);
    }
    return result(iterations * CALL_TREE_CALLS_PER_ITERATION, checksum);
}

var RECEIVER_POOL_MASK = 63;

function Stepper(step) {
    this.step = step;
}

Stepper.prototype.advance = function (value) {
    return value + this.step;
};

function buildStepperPool() {
    var pool = [];
    for (var i = 0; i <= RECEIVER_POOL_MASK; i++) {
        pool.push(new Stepper(1));
    }
    return pool;
}

/*
 * Prototype-dispatched method call on a receiver whose identity changes every
 * iteration. The body reads `this.step`, so the callee cannot be replaced by a
 * constant even though every pooled receiver holds the same step.
 */
function runPrototypeMethodCall(iterations) {
    var pool = buildStepperPool();
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += pool[i & RECEIVER_POOL_MASK].advance(i);
    }
    return result(iterations, checksum);
}

function incrementByOne(value) {
    return value + 1;
}

function incrementByTwoMinusOne(value) {
    return value + 2 - 1;
}

function incrementThroughLocal(value) {
    var step = 1;
    return value + step;
}

/*
 * A call site whose callee identity rotates between distinct functions. Each
 * returns `value + 1`, so the checksum stays closed form while the site itself
 * stays polymorphic. The index uses a mask rather than a remainder so the case
 * measures dispatch instead of division.
 */
function runPolymorphicCallSite(iterations) {
    var callees = [
        incrementByOne,
        incrementByTwoMinusOne,
        incrementThroughLocal,
        incrementByOne
    ];
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += callees[i & 3](i);
    }
    return result(iterations, checksum);
}

function makeStepClosure(step) {
    return function (value) {
        return value + step;
    };
}

/*
 * Capturing closures selected from a runtime-built table. Each closure reads a
 * captured cell rather than a literal, so the call cannot collapse into the
 * surrounding loop.
 */
function runCapturingClosureCall(iterations) {
    var closures = [
        makeStepClosure(1),
        makeStepClosure(1),
        makeStepClosure(1),
        makeStepClosure(1)
    ];
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += i + closures[i & 3](0);
    }
    return result(iterations, checksum);
}

/*
 * Receivers deliberately built with three different storage shapes, so a named
 * read cannot assume one layout. Every shape holds `step`, `carry`, and `rest`
 * at a different position, and each iteration reads all three, so a two-entry
 * receiver cache cannot cover the site.
 */
function buildHeterogeneousPool() {
    var pool = [];
    for (var i = 0; i <= RECEIVER_POOL_MASK; i++) {
        var shape = i % 3;
        if (shape === 0) {
            pool.push({ step: 1, carry: 0, rest: 0, left: i });
        } else if (shape === 1) {
            pool.push({ left: i, carry: 0, step: 1, rest: 0 });
        } else {
            pool.push({ left: i, rest: 0, extra: i, carry: 0, step: 1 });
        }
    }
    return pool;
}

function runHeterogeneousPropertyRead(iterations) {
    var pool = buildHeterogeneousPool();
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        var receiver = pool[i & RECEIVER_POOL_MASK];
        checksum += i + receiver.step + receiver.carry + receiver.rest;
    }
    return result(iterations * 3, checksum);
}

var STRING_KEY_MASK = 1023;

function buildStringKeys() {
    var keys = [];
    for (var i = 0; i <= STRING_KEY_MASK; i++) {
        keys.push("entry" + i);
    }
    return keys;
}

/*
 * Computed string-key read and write against an object that grows past every
 * small-storage threshold. This is the ordinary dictionary path that a
 * hash-map workload actually exercises.
 */
function runStringKeyMapChurn(iterations) {
    var keys = buildStringKeys();
    var table = {};
    for (var i = 0; i < iterations; i++) {
        var key = keys[i & STRING_KEY_MASK];
        table[key] = (table[key] || 0) + 1;
    }
    var checksum = 0;
    for (var name in table) {
        checksum += table[name];
    }
    return result(iterations * 2, checksum);
}

function run(caseId, iterations) {
    if (caseId === "recursive_call_tree") return runRecursiveCallTree(iterations);
    if (caseId === "prototype_method_call") return runPrototypeMethodCall(iterations);
    if (caseId === "polymorphic_call_site") return runPolymorphicCallSite(iterations);
    if (caseId === "capturing_closure_call") return runCapturingClosureCall(iterations);
    if (caseId === "heterogeneous_property_read") {
        return runHeterogeneousPropertyRead(iterations);
    }
    if (caseId === "string_key_map_churn") return runStringKeyMapChurn(iterations);
    fail("unknown case " + caseId);
}

if (scriptArgs.length !== 3) {
    fail("expected CASE ITERATIONS arguments");
}

var caseId = scriptArgs[1];
var iterations = parseIterations(scriptArgs[2]);
var benchmarkResult = run(caseId, iterations);
var benchmarkOutput = "QJS_BENCH_RESULT " + JSON.stringify({
    case_id: caseId,
    iterations: iterations,
    operations: benchmarkResult.operations,
    checksum: benchmarkResult.checksum
});
if (typeof console !== "undefined") {
    console.log(benchmarkOutput);
}
benchmarkOutput;
