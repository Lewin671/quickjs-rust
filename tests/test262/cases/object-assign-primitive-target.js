// Derived from: test/built-ins/Object/assign/OnlyOneArgument.js
var target = "a";
var result = Object.assign(target);

if (typeof result !== "object") { throw; }
if (result.valueOf() !== "a") { throw; }
