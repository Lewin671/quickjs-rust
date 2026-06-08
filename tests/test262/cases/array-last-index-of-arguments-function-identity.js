// Derived from: test/built-ins/Array/prototype/lastIndexOf/15.4.4.15-2-17.js
var targetObj = function() {};
var func = function(a, b) {
  arguments[2] = function() {};
  return Array.prototype.lastIndexOf.call(arguments, targetObj) === 1 &&
    Array.prototype.lastIndexOf.call(arguments, arguments[2]) === -1;
};

if (!func(0, targetObj)) {
  throw "Array.prototype.lastIndexOf should compare function arguments by reference identity";
}
