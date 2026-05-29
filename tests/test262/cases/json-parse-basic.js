// Derived from: test/built-ins/JSON/parse/15.12.2-2-1.js
if (typeof JSON !== "object") { throw; }
if (JSON.parse.length !== 2) { throw; }
if (JSON.parse("null") !== null) { throw; }
if (JSON.parse("true") !== true) { throw; }
if (JSON.parse("-12.5e2") !== -1250) { throw; }
if (JSON.parse("\"text\"") !== "text") { throw; }
var value = JSON.parse("{\"a\":1,\"b\":[2]}");
if (value.a !== 1) { throw; }
if (value.b[0] !== 2) { throw; }
