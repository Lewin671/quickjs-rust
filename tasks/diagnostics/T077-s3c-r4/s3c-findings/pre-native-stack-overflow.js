// PRE-EXISTING on S3b and S3c: exceeding native depth aborts the process.
// Expected eventual policy: catchable RangeError. S3d owns that policy.
function g(n) { try { return n === 0 ? 0 : g(n - 1) + 1; } catch (e) { throw e; } }
try { print('R ' + g(100000)); } catch (e) { print(e.name); }
