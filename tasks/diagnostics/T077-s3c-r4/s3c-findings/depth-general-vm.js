// T077 S3c review: release native-stack depth regression on the general VM path.
// A recursive function the register tiers do not admit (here: try/catch) recurses N times.
// Release: base e60e7cfa completes up to N~1864; candidate c80ac690 aborts (SIGABRT,
// "thread 'main' has overflowed its stack") from N~1670. Debug profile: base ~509, cand ~454.
// Usage: qjs depth-general-vm.js            (N = 1750, release)
var N = 1750;
function g(n) { try { return n === 0 ? 0 : g(n - 1) + 1; } catch (e) { throw e; } }
print("R " + g(N));
