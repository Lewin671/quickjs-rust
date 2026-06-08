// Derived from: test/built-ins/Array/prototype/copyWithin/coerced-values-start-change-start.js
// Derived from: test/built-ins/Array/prototype/copyWithin/coerced-values-start-change-target.js
var shortened = [0, 1, 2, 3];
shortened.copyWithin(0, {
  valueOf: function() {
    shortened.length = 2;
    return 3;
  }
});
if (shortened.length !== 2 || shortened.hasOwnProperty("0") || shortened[1] !== 1) {
  throw "Array.prototype.copyWithin should observe start coercion side effects";
}

var proto = { 3: 9 };
var inherited = [0, 1, 2, 3];
Object.setPrototypeOf(inherited, proto);
Array.prototype.copyWithin.call(inherited, 0, {
  valueOf: function() {
    inherited.length = 2;
    return 3;
  }
});
if (inherited.length !== 2 || inherited[0] !== 9 || inherited[1] !== 1) {
  throw "Array.prototype.copyWithin should read inherited values after start coercion side effects";
}
