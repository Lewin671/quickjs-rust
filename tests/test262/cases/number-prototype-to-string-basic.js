// Derived from: test/built-ins/Number/prototype/toString/S15.7.4.2_A1_T01.js
if (Number.prototype.toString() !== "0") {
  throw "expected Number.prototype.toString() to return 0";
}
if ((255).toString(16) !== "ff") {
  throw "expected Number.prototype.toString to support radix 16";
}
if ((new Number(7)).toString() !== "7") {
  throw "expected Number object toString to use wrapped value";
}
