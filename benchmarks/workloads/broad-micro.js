/*
 * First-party broad black-box workload for quickjs-rust and QuickJS-NG.
 *
 * The portfolio intentionally mixes optimized counted-loop shapes with
 * holdouts that differ by scope, arity, expression order, control flow,
 * receiver stability, mutation, builtins, strings, and allocation. The host
 * measures process wall time. This program only supplies deterministic
 * operation counts and correctness checksums; it contains no timer.
 */

function fail(message) {
    throw new Error("broad benchmark workload: " + message);
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

function addOne(value) {
    return value + 1;
}

function addTwo(value, increment) {
    return value + increment;
}

function runPlainFunctionCall(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += addOne(i);
    }
    return result(iterations, checksum);
}

function runMethodCall(iterations) {
    var receiver = { addOne: function (value) { return value + 1; } };
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += receiver.addOne(i);
    }
    return result(iterations, checksum);
}

function makeCapturedReader() {
    var captured = 7;
    return function (value) { return value + captured; };
}

function runCapturedRead(iterations) {
    var read = makeCapturedReader();
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += read(i);
    }
    return result(iterations, checksum);
}

function makeCapturedWriter() {
    var captured = 0;
    return function () { captured += 1; return captured; };
}

function runCapturedWrite(iterations) {
    var write = makeCapturedWriter();
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += write();
    }
    return result(iterations, checksum);
}

function addManyLocals(value) {
    var a01 = 1, a02 = 2, a03 = 3, a04 = 4;
    var a05 = 5, a06 = 6, a07 = 7, a08 = 8;
    var a09 = 9, a10 = 10, a11 = 11, a12 = 12;
    var a13 = 13, a14 = 14, a15 = 15, a16 = 16;
    return value + a01 + a02 + a03 + a04 + a05 + a06 + a07 + a08 +
        a09 + a10 + a11 + a12 + a13 + a14 + a15 + a16;
}

function runManyLocalsCall(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += addManyLocals(i);
    }
    return result(iterations, checksum);
}

function runPropertyRead(iterations) {
    var object = { a: 1, b: 2, c: 3 };
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += object.a;
        checksum += object.b;
        checksum += object.c;
    }
    return result(iterations * 3, checksum);
}

function runArrayRead(iterations) {
    var array = [1, 2, 3, 4];
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += array[0];
        checksum += array[1];
        checksum += array[2];
        checksum += array[3];
    }
    return result(iterations * 4, checksum);
}

function runFunctionCallTwoArgs(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += addTwo(i, 1);
    }
    return result(iterations, checksum);
}

function runFunctionCallReordered(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum = addOne(i) + checksum;
    }
    return result(iterations, checksum);
}

function runDynamicMethodCall(iterations) {
    var first = { addOne: function (value) { return value + 1; } };
    var second = { addOne: function (value) { return value + 1; } };
    var receiver;
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        receiver = (i & 1) === 0 ? first : second;
        checksum += receiver.addOne(i);
    }
    return result(iterations, checksum);
}

function runLocalRead(iterations) {
    var first = 1;
    var second = 2;
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += first;
        checksum += second;
    }
    return result(iterations * 2, checksum);
}

var broadGlobalOne = 1;

function runGlobalRead(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += broadGlobalOne;
    }
    return result(iterations, checksum);
}

function runPropertyDynamicRead(iterations) {
    var object = { a: 1, b: 2, c: 3 };
    var first = "a", second = "b", third = "c";
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += object[first];
        checksum += object[second];
        checksum += object[third];
    }
    return result(iterations * 3, checksum);
}

function runPropertyWrite(iterations) {
    var object = { a: 0, b: 0, c: 0 };
    for (var i = 0; i < iterations; i++) {
        object.a = i;
        object.b = i + 1;
        object.c = i + 2;
    }
    return result(iterations * 3, object.a + object.b + object.c);
}

function runArrayDynamicRead(iterations) {
    var array = [1, 2, 3, 4];
    var first = 0, second = 1, third = 2, fourth = 3;
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += array[first];
        checksum += array[second];
        checksum += array[third];
        checksum += array[fourth];
    }
    return result(iterations * 4, checksum);
}

function runArrayWrite(iterations) {
    var array = [0, 0, 0, 0];
    for (var i = 0; i < iterations; i++) {
        array[0] = i + 1;
        array[1] = i + 1;
        array[2] = i + 1;
        array[3] = i + 1;
    }
    return result(iterations * 4, array[0] + array[1] + array[2] + array[3]);
}

function runEmptyLoop(iterations) {
    var i;
    for (i = 0; i < iterations; i++) {
    }
    return result(iterations, i);
}

function runBranchArithmetic(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        if ((i & 1) === 0) {
            checksum += 1;
        } else {
            checksum += 1;
        }
    }
    return result(iterations, checksum);
}

function runMathAbs(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += Math.abs(-1);
    }
    return result(iterations, checksum);
}

function runArrayIndexOf(iterations) {
    var array = [1, 2, 3, 4];
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += array.indexOf(3);
    }
    return result(iterations, checksum);
}

function runStringSlice(iterations) {
    var text = "the quick brown fox";
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += text.slice(1, 4).length;
    }
    return result(iterations, checksum);
}

function runObjectAllocation(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        var object = { a: 1, b: 2 };
        checksum += object.a + object.b;
    }
    return result(iterations, checksum);
}

function runArrayAllocation(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        var array = [1, 2, 3];
        checksum += array[2];
    }
    return result(iterations, checksum);
}

function runClosureAllocationCall(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        var add = function (value) { return value + 1; };
        checksum += add(0);
    }
    return result(iterations, checksum);
}

function run(caseId, iterations) {
    if (caseId === "plain_function_call") return runPlainFunctionCall(iterations);
    if (caseId === "method_call") return runMethodCall(iterations);
    if (caseId === "captured_read") return runCapturedRead(iterations);
    if (caseId === "captured_write") return runCapturedWrite(iterations);
    if (caseId === "many_locals_call") return runManyLocalsCall(iterations);
    if (caseId === "property_read") return runPropertyRead(iterations);
    if (caseId === "array_read") return runArrayRead(iterations);
    if (caseId === "function_call_two_args") return runFunctionCallTwoArgs(iterations);
    if (caseId === "function_call_reordered") return runFunctionCallReordered(iterations);
    if (caseId === "dynamic_method_call") return runDynamicMethodCall(iterations);
    if (caseId === "local_read") return runLocalRead(iterations);
    if (caseId === "global_read") return runGlobalRead(iterations);
    if (caseId === "property_dynamic_read") return runPropertyDynamicRead(iterations);
    if (caseId === "property_write") return runPropertyWrite(iterations);
    if (caseId === "array_dynamic_read") return runArrayDynamicRead(iterations);
    if (caseId === "array_write") return runArrayWrite(iterations);
    if (caseId === "empty_loop") return runEmptyLoop(iterations);
    if (caseId === "branch_arithmetic") return runBranchArithmetic(iterations);
    if (caseId === "math_abs") return runMathAbs(iterations);
    if (caseId === "array_index_of") return runArrayIndexOf(iterations);
    if (caseId === "string_slice") return runStringSlice(iterations);
    if (caseId === "object_allocation") return runObjectAllocation(iterations);
    if (caseId === "array_allocation") return runArrayAllocation(iterations);
    if (caseId === "closure_allocation_call") return runClosureAllocationCall(iterations);
    fail("unknown case " + caseId);
}

if (scriptArgs.length !== 3) {
    fail("expected CASE ITERATIONS arguments");
}

var caseId = scriptArgs[1];
var iterations = parseIterations(scriptArgs[2]);
var benchmarkResult;
if (caseId === "top_level_function_call") {
    var topLevelChecksum = 0;
    for (var topLevelIndex = 0; topLevelIndex < iterations; topLevelIndex++) {
        topLevelChecksum += addOne(topLevelIndex);
    }
    benchmarkResult = result(iterations, topLevelChecksum);
} else {
    benchmarkResult = run(caseId, iterations);
}
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
