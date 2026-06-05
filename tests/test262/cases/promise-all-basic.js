// Derived from: test/built-ins/Promise/all/S25.4.4.1_A1.1_T1.js
if (typeof Promise.all !== "function") {
  throw "Promise.all should be a function";
}
if (Promise.all.length !== 1) {
  throw "Promise.all.length should be 1";
}
if (Promise.propertyIsEnumerable("all")) {
  throw "Promise.all should not be enumerable";
}
