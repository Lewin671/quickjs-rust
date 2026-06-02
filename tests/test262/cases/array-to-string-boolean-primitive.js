// Derived from: test/built-ins/Array/prototype/toString/call-with-boolean.js
if (Array.prototype.toString.call(true) !== "[object Boolean]") { throw; }
if (Array.prototype.toString.call(false) !== "[object Boolean]") { throw; }
