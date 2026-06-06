// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-625gs.js

Object.defineProperty(Object.prototype, "prop", {
  value: 1001,
  writable: false,
  enumerable: false,
  configurable: false,
});
var prop = 1002;

if (!(this.hasOwnProperty("prop") && prop === 1002 && this.prop === 1002)) {
  throw "expected global var to shadow Object.prototype property";
}
