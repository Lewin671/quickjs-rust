/*
 * First-party black-box micro workload for quickjs-rust and QuickJS-NG.
 *
 * Behavior-family provenance: third_party/quickjs-ng/tests/microbench.js at
 * f7830186043e4488f2998759d60a514faf07cbc9. This is a repository-authored
 * workload/protocol, not a copy of the upstream harness or its score.
 *
 * The host measures process wall time. This program only supplies a validated
 * operation count and deterministic correctness checksum; it is not a timer.
 */

function fail(message) {
    throw new Error("benchmark workload: " + message);
}

function parseIterations(text) {
    var value = Number(text);
    if (!Number.isFinite(value) || value < 0 || Math.floor(value) !== value) {
        fail("iterations must be a non-negative integer");
    }
    return value;
}

function addOne(value) {
    return value + 1;
}

function runPlainFunctionCall(iterations) {
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += addOne(i);
    }
    return { operations: iterations, checksum: checksum };
}

function runMethodCall(iterations) {
    var receiver = {
        addOne: function (value) { return value + 1; }
    };
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += receiver.addOne(i);
    }
    return { operations: iterations, checksum: checksum };
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
    return { operations: iterations, checksum: checksum };
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
    return { operations: iterations, checksum: checksum };
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
    return { operations: iterations, checksum: checksum };
}

function runPropertyRead(iterations) {
    var object = { a: 1, b: 2, c: 3 };
    var checksum = 0;
    for (var i = 0; i < iterations; i++) {
        checksum += object.a;
        checksum += object.b;
        checksum += object.c;
    }
    return { operations: iterations * 3, checksum: checksum };
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
    return { operations: iterations * 4, checksum: checksum };
}

function run(caseId, iterations) {
    if (caseId === "plain_function_call") {
        return runPlainFunctionCall(iterations);
    }
    if (caseId === "method_call") {
        return runMethodCall(iterations);
    }
    if (caseId === "captured_read") {
        return runCapturedRead(iterations);
    }
    if (caseId === "captured_write") {
        return runCapturedWrite(iterations);
    }
    if (caseId === "many_locals_call") {
        return runManyLocalsCall(iterations);
    }
    if (caseId === "property_read") {
        return runPropertyRead(iterations);
    }
    if (caseId === "array_read") {
        return runArrayRead(iterations);
    }
    fail("unknown case " + caseId);
}

if (scriptArgs.length !== 3) {
    fail("expected CASE ITERATIONS arguments");
}

var caseId = scriptArgs[1];
var iterations = parseIterations(scriptArgs[2]);
var result = run(caseId, iterations);
var benchmarkOutput = "QJS_BENCH_RESULT " + JSON.stringify({
    case_id: caseId,
    iterations: iterations,
    operations: result.operations,
    checksum: result.checksum
});
if (typeof console !== "undefined") {
    console.log(benchmarkOutput);
}
benchmarkOutput;
