// Derived from: test/built-ins/Object/prototype/propertyIsEnumerable/S15.2.4.7_A1_T1.js
var proto = { root: 1 };
var object = Object.create(proto);
object.own = 2;
if (!object.propertyIsEnumerable("own")) { throw; }
if (object.propertyIsEnumerable("root")) { throw; }
