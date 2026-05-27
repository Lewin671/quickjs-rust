// Derived from: test/built-ins/Array/prototype/includes/search-found-returns-true.js
var array = [42, "test262", null, undefined, true, false, 0, -1, ""];
if (!array.includes(42)) { throw; }
if (!array.includes("test262")) { throw; }
if (!array.includes(undefined)) { throw; }
if (!array.includes(false)) { throw; }
