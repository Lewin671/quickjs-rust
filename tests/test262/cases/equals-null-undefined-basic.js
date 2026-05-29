// Derived from: test/language/expressions/equals/S11.9.1_A6.2_T1.js
if ((null == undefined) !== true) { throw; }
if ((undefined == null) !== true) { throw; }
if ((null != undefined) !== false) { throw; }
