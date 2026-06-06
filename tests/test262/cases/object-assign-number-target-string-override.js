// Derived from: test/built-ins/Object/assign/Override-notstringtarget.js
var target = 12;
var result = Object.assign(target, "aaa", "bb2b", "1c");

if (Object.getOwnPropertyNames(result).length !== 4) {
  throw "expected boxed number target to expose only assigned properties";
}
if (result[0] !== "1" || result[1] !== "c" || result[2] !== "2" || result[3] !== "b") {
  throw "expected later string sources to override previous indexed properties";
}
