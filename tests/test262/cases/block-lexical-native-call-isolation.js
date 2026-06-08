// Derived from: test/built-ins/Set/prototype/union/result-order.js
// Derived from: test/language/statements/block/scope-lex-open.js
{
  const s1 = new Set([1, 2]);
  const s2 = new Set([2, 3]);
  var first = [...s1.union(s2)].join("|");
}

{
  const s1 = new Set([2, 3]);
  const s2 = new Set([1, 2]);
  var second = [...s1.union(s2)].join("|");
}

{
  const s1 = new Set([1, 2]);
  const s2 = new Set([3]);
  var third = [...s1.union(s2)].join("|");
}

{
  const s1 = new Set([3]);
  const s2 = new Set([1, 2]);
  var fourth = [...s1.union(s2)].join("|");
}

if (first !== "1|2|3") {
  throw "expected first block union order";
}
if (second !== "2|3|1") {
  throw "expected second block union order";
}
if (third !== "1|2|3") {
  throw "expected third block union order";
}
if (fourth !== "3|1|2") {
  throw "expected fourth block union order";
}
