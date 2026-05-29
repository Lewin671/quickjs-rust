// Derived from: test/built-ins/String/S15.5.2.1_A1_T2.js
var value = new String();
if (typeof value !== "object") { throw; }
if (value.constructor !== String) { throw; }
if ((value == "") !== true) { throw; }
if ((value !== "") !== true) { throw; }
