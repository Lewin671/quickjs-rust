// Derived from: test/built-ins/JSON/stringify/value-primitive-top-level.js
if (JSON.stringify.length !== 3) { throw; }
if (JSON.stringify(null) !== "null") { throw; }
if (JSON.stringify(true) !== "true") { throw; }
if (JSON.stringify("text") !== "\"text\"") { throw; }
if (JSON.stringify(["x", undefined, NaN, Infinity]) !== "[\"x\",null,null,null]") { throw; }
if (JSON.stringify({a: 1, b: [true, null], c: undefined}) !== "{\"a\":1,\"b\":[true,null]}") { throw; }
if (JSON.stringify(undefined) !== undefined) { throw; }
