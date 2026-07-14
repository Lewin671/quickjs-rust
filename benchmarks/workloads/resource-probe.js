/* Deterministic correctness probes for process-latency and peak-RSS lanes. */

function fail(message) {
    throw new Error("resource benchmark workload: " + message);
}

function parseIterations(text) {
    var value = Number(text);
    if (!Number.isFinite(value) || value < 1 || Math.floor(value) !== value) {
        fail("iterations must be a positive integer");
    }
    return value;
}

function run(caseId, iterations) {
    var checksum = 0;
    if (caseId === "fresh_process_probe") {
        for (var i = 0; i < iterations; i++) {
            checksum += 7;
        }
        return { operations: iterations, checksum: checksum };
    }
    if (caseId === "peak_rss_probe") {
        var retained = [];
        for (var j = 0; j < iterations; j++) {
            retained.push(j);
            checksum += 3;
        }
        if (retained.length !== iterations) {
            fail("retained allocation length mismatch");
        }
        return { operations: iterations, checksum: checksum };
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
