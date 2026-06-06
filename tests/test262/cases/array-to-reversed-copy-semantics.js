// Derived from: test/built-ins/Array/prototype/toReversed/get-descending-order.js
// Derived from: test/built-ins/Array/prototype/toReversed/holes-not-preserved.js
var order = [];
var arrayLike = {
  length: 3,
  get 0() {
    order.push(0);
    return "a";
  },
  get 1() {
    order.push(1);
    return "b";
  },
  get 2() {
    order.push(2);
    return "c";
  }
};

var result = Array.prototype.toReversed.call(arrayLike);
if (order.join(",") !== "2,1,0" || result.join("") !== "cba") {
  throw "Array.prototype.toReversed should read source indexes in descending order";
}

var array = [0, , 2, , 4];
Array.prototype[3] = 3;
var reversed = array.toReversed();
delete Array.prototype[3];

if (reversed.join("|") !== "4|3|2||0") {
  throw "Array.prototype.toReversed should copy holes through ordinary Get";
}
if (!reversed.hasOwnProperty("1") || !reversed.hasOwnProperty("3")) {
  throw "Array.prototype.toReversed result should contain own properties for copied holes";
}
