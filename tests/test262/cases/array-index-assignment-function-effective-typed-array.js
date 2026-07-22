// Derived from: test/built-ins/TypedArrayConstructors/internals/Set/key-is-canonical-invalid-index-prototype-chain-set.js
var original = Object.getPrototypeOf(Function.prototype);
var typedArray = new Uint8Array(0);

Object.setPrototypeOf(Function.prototype, typedArray);
function link() {}
var bridge = Object.create(link);
var array = [];
Object.setPrototypeOf(array, bridge);
array[0] = 23;
Object.setPrototypeOf(Function.prototype, original);

if (Object.prototype.hasOwnProperty.call(array, "0")) {
  throw "expected the zero-length TypedArray exotic Set to stop assignment";
}
if (array.length !== 0) {
  throw "expected no Array DefineOwnProperty after the invalid typed-array index";
}
