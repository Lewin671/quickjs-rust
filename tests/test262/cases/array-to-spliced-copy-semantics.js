// Derived from: test/built-ins/Array/prototype/toSpliced/discarded-element-not-read.js
// Derived from: test/built-ins/Array/prototype/toSpliced/elements-read-in-order.js
// Derived from: test/built-ins/Array/prototype/toSpliced/start-and-deleteCount-missing.js
var order = [];
var arrayLike = {
  get 0() {
    order.push(0);
    return "a";
  },
  get 1() {
    order.push(1);
    return "b";
  },
  get 2() {
    throw "discarded element should not be read";
  },
  get 3() {
    order.push(3);
    return "c";
  },
  length: 4
};

var result = Array.prototype.toSpliced.call(arrayLike, 2, 1);
if (result.join("|") !== "a|b|c") {
  throw "Array.prototype.toSpliced should copy prefix and suffix values";
}
if (order.join(",") !== "0,1,3") {
  throw "Array.prototype.toSpliced should read retained source indexes in order";
}

var unchanged = [1, 2, 3].toSpliced();
if (unchanged.join() !== "1,2,3") {
  throw "Array.prototype.toSpliced should not delete when start is missing";
}
