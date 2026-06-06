// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-325.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-325-1.js
let accessed = false;
let object = {};
Object.defineProperty(object, "0", {
  get: function() {
    accessed = true;
    return 12;
  },
});
if (object[0] !== 12 || !accessed) {
  throw "expected ordinary accessor getter to update caller binding";
}

accessed = false;
let argObj = (function(a, b, c) {
  return arguments;
})(1, 2, 3);
Object.defineProperty(argObj, "0", {
  get: function() {
    accessed = true;
    return 12;
  },
});
if (argObj[0] !== 12 || !accessed) {
  throw "expected arguments accessor getter to update caller binding";
}
