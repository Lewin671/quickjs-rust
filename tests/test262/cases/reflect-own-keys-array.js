// Derived from: test/built-ins/Reflect/ownKeys/ownKeys.js
var keys = Reflect.ownKeys([1, 2]);
if (keys.length !== 3) {
  throw "expected array index keys and length";
}
if (keys[0] !== "0" || keys[1] !== "1" || keys[2] !== "length") {
  throw "expected array own keys";
}
