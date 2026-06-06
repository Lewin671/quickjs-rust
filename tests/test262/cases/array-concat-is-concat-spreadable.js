// Derived from: test/built-ins/Array/prototype/concat/is-concat-spreadable-val-falsey.js
// Derived from: test/built-ins/Array/prototype/concat/is-concat-spreadable-val-truthy.js
// Derived from: test/built-ins/Array/prototype/concat/is-concat-spreadable-val-undefined.js
// Derived from: test/built-ins/Array/prototype/concat/Array.prototype.concat_spreadable-sparse-object.js
// Derived from: test/built-ins/Array/prototype/concat/Array.prototype.concat_spreadable-getter-throws.js
var array = [1, 2];
array[Symbol.isConcatSpreadable] = false;
var kept = [].concat(array);
if (kept.length !== 1) {
  throw "expected false isConcatSpreadable array length";
}
if (kept[0] !== array) {
  throw "expected false isConcatSpreadable array to be kept";
}

array[Symbol.isConcatSpreadable] = null;
kept = [].concat(array);
if (kept.length !== 1 || kept[0] !== array) {
  throw "expected null isConcatSpreadable array to be kept";
}

array[Symbol.isConcatSpreadable] = 0;
kept = [].concat(array);
if (kept.length !== 1 || kept[0] !== array) {
  throw "expected zero isConcatSpreadable array to be kept";
}

array[Symbol.isConcatSpreadable] = undefined;
var spread = [].concat(array);
if (spread.length !== 2 || spread[0] !== 1 || spread[1] !== 2) {
  throw "expected undefined isConcatSpreadable array to use IsArray";
}

var object = { length: 3, 0: "a", 2: "c" };
object[Symbol.isConcatSpreadable] = true;
spread = [].concat(object);
if (spread.length !== 3) {
  throw "expected object spread length";
}
if (spread[0] !== "a" || spread[2] !== "c") {
  throw "expected object spread indexed values";
}
if (spread.hasOwnProperty("1")) {
  throw "expected object spread to preserve hole";
}

object[Symbol.isConcatSpreadable] = 86;
spread = [].concat(object);
if (spread.length !== 3 || spread[0] !== "a" || spread[2] !== "c") {
  throw "expected truthy isConcatSpreadable object to spread";
}

var marker = {};
var poisoned = {};
Object.defineProperty(poisoned, Symbol.isConcatSpreadable, {
  get: function () {
    throw marker;
  }
});
var caught = false;
try {
  [].concat(poisoned);
} catch (error) {
  caught = error === marker;
}
if (!caught) {
  throw "expected isConcatSpreadable getter error";
}
