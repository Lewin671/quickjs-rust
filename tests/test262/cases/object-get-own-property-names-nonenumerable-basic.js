// Derived from: test/built-ins/Object/prototype/S15.2.4_A2.js
var names = Object.getOwnPropertyNames(Object.prototype);
if (names.length !== 4) { throw; }
if (names[0] !== "constructor") { throw; }
if (names[1] !== "hasOwnProperty") { throw; }
if (names[2] !== "toString") { throw; }
if (names[3] !== "valueOf") { throw; }
