// Derived from: test/built-ins/String/numeric-properties.js
var value = new String("abc");
if (value[0] !== "a" || value[1] !== "b" || value[2] !== "c") { throw; }
var descriptor = Object.getOwnPropertyDescriptor(value, "1");
if (descriptor.value !== "b") { throw; }
if (descriptor.enumerable !== true) { throw; }
if (descriptor.writable !== false) { throw; }
if (descriptor.configurable !== false) { throw; }
