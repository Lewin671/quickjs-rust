// Derived from: test/built-ins/Array/prototype/join/S15.4.4.5_A2_T1.js
var object = {};
object.join = Array.prototype.join;
if (object.join() !== "") { throw; }
object.length = null;
if (object.join() !== "") { throw; }
