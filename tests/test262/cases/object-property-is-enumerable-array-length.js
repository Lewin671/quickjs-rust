// Derived from: test/built-ins/Array/S15.4.5.2_A1_T1.js
if ([1, 2].propertyIsEnumerable("0") !== true) { throw; }
if ([1, 2].propertyIsEnumerable("length") !== false) { throw; }
