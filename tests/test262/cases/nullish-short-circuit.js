// Derived from: test/language/expressions/coalesce/short-circuit-prevents-evaluation.js
if ((42 ?? missing) !== 42) {
  throw;
}

if ((undefined ?? 42 ?? missing) !== 42) {
  throw;
}

if ((null ?? 0 ?? missing) !== 0) {
  throw;
}
