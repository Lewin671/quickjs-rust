// Derived from: test/built-ins/Object/entries/order-after-define-property-with-function.js
var fn = function() {};
fn.a = 1;
Object.defineProperty(fn, "name", { enumerable: true });
var entries = Object.entries(fn);
if (entries.length !== 2) { throw; }
if (entries[0][0] !== "name") { throw; }
if (entries[1][0] !== "a") { throw; }
