// Derived from: test/built-ins/Array/prototype/join/call-with-boolean.js
if (Array.prototype.join.call(true) !== "") { throw; }
if (Array.prototype.join.call(false) !== "") { throw; }
