// Derived from: test/built-ins/Reflect/ownKeys/ownKeys.js
var keys = Reflect.ownKeys({ a: 1, b: 2 });
if (keys.length !== 2) {
  throw "expected two keys";
}
if (keys[0] !== "a" || keys[1] !== "b") {
  throw "expected own string keys";
}
