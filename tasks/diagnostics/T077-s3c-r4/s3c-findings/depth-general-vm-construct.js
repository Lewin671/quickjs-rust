// T077 S3c review: same native-stack depth regression on the direct construct fallback path.
// Release: base e60e7cfa / S3b 8322fc98 complete up to N~1815; candidate c80ac690 aborts (SIGABRT,
// stack overflow) from N~1645. Debug profile: base ~497, candidate ~442.
var N = 1730;
function C(n) { try { this.c = n === 0 ? 0 : new C(n - 1).c + 1; } catch (e) { throw e; } }
print("R " + new C(N).c);
