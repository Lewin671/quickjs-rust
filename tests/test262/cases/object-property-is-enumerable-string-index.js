// Derived from: test/built-ins/String/prototype/S15.5.4_A1.js
if ("abc".propertyIsEnumerable("1") !== true) { throw; }
if ("abc".propertyIsEnumerable("length") !== false) { throw; }
