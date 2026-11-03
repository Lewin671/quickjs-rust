(function () {
  var log = [];
  function A(n) { return { async [Symbol.asyncDispose]() { log.push(n + "1"); await null; log.push(n + "2"); } }; }
  function S(n) { return { [Symbol.dispose]() { log.push(n); } }; }
  var out = [];
  (async function () { await using a = A("a"), b = A("b"); log.push("body"); })();
  out.push(log.join()); log = [];
  (async function () { using s = S("s"); await using n = null; })();
  out.push(log.join()); log = [];
  (async function () { using s = S("s"); await using n = null, m = undefined; log.push("body"); })();
  out.push(log.join()); log = [];
  (async function () { for (await using x of [A("p"), A("q")]) log.push("it"); })();
  out.push(log.join()); log = [];
  (async function () { await using a = { [Symbol.dispose]() { log.push("f"); } }, b = A("b"); })();
  out.push(log.join());
  return out.join(" | ");
})()
