// Derived from: test/built-ins/Array/prototype/with/holes-not-preserved.js
// Derived from: test/built-ins/Array/prototype/with/no-get-replaced-index.js
var array = [0, , 2, , 4];
Array.prototype[3] = 3;

var result = array.with(2, 6);
delete Array.prototype[3];

if (result.join("|") !== "0||6|3|4") {
  throw "expected Array.prototype.with to copy holes through ordinary Get";
}
if (!result.hasOwnProperty("1") || !result.hasOwnProperty("3")) {
  throw "expected Array.prototype.with result to contain own properties for copied holes";
}

var throwing = [0, 1, 2, 3];
Object.defineProperty(throwing, "2", {
  get: function() {
    throw "should not get replaced index";
  }
});

var replaced = throwing.with(2, 6);
if (replaced.join("|") !== "0|1|6|3") {
  throw "expected Array.prototype.with to avoid reading the replaced index";
}
