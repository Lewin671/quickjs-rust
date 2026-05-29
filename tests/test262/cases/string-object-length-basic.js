// Derived from: test/built-ins/String/S15.5.5.1_A4_T2.js
var value = new String("globglob");
if (value.hasOwnProperty("length") !== true) { throw; }
if (value.length !== 8) { throw; }
try { value.length = -1; } catch (error) {}
if (value.length !== 8) { throw; }
