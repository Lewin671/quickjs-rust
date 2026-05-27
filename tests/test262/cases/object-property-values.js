// Derived from: test/language/expressions/object/S11.1.5_A2.js
var x = true;
var object = { prop: x };
if (object.prop !== x) { throw; }

var y = null;
var other = { prop: y };
if (other.prop !== y) { throw; }
