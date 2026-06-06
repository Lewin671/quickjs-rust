// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-2-39.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-2-40.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-2-41.js

var arrayKeyObject = {};
Object.defineProperty(arrayKeyObject, [1, 2], {});
if (!arrayKeyObject.hasOwnProperty("1,2")) {
  throw "expected array key to convert to string";
}

var stringKeyObject = {};
Object.defineProperty(stringKeyObject, new String("Hello"), {});
if (!stringKeyObject.hasOwnProperty("Hello")) {
  throw "expected String object key to convert to string";
}

var booleanKeyObject = {};
Object.defineProperty(booleanKeyObject, new Boolean(false), {});
if (!booleanKeyObject.hasOwnProperty("false")) {
  throw "expected Boolean object key to convert to string";
}
