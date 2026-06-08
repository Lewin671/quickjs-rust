// Derived from: test/built-ins/Array/prototype/splice/S15.4.4.12_A2_T1.js
var object = {
  0: 0,
  1: 1,
  2: 2,
  3: 3,
  length: 4
};
var removed = Array.prototype.splice.call(object, 0, 3, 4, 5);

if (removed.length !== 3 || removed[0] !== 0 || removed[1] !== 1 || removed[2] !== 2) {
  throw "Array.prototype.splice should return deleted elements from an object receiver";
}
if (object.length !== 3 || object[0] !== 4 || object[1] !== 5 || object[2] !== 3 || object[3] !== undefined) {
  throw "Array.prototype.splice should mutate an object receiver generically";
}
