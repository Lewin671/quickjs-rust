// Derived from: test/built-ins/Object/prototype/toString/direct-invocation.js
if (Object.prototype.toString() !== "[object Object]") { throw; }
if (({}).toString() !== "[object Object]") { throw; }
