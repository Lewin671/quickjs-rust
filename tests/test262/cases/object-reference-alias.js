// Derived from: test/language/types/reference/S8.7_A1.js
var obj = {};
var objRef = obj;
objRef.oneProperty = -1;
obj.oneProperty = true;
if (objRef.oneProperty !== true) { throw; }
