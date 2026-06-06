// Derived from: test/built-ins/Object/assign/target-Array.js
var target = [7, 8, 9];
var result = Object.assign(target, [1]);
if (result !== target) { throw; }
if (result[0] !== 1) { throw; }
if (result[1] !== 8) { throw; }
if (result[2] !== 9) { throw; }

result = Object.assign(target, { 1: 2, length: 2 });
if (result !== target) { throw; }
if (target.length !== 2) { throw; }
if (target[0] !== 1) { throw; }
if (target[1] !== 2) { throw; }

result = Object.assign(target, { 2: 0, extra: "ok" });
if (result !== target) { throw; }
if (target.length !== 3) { throw; }
if (target[2] !== 0) { throw; }
if (target.extra !== "ok") { throw; }
