// Derived from: test/built-ins/Promise/race/S25.4.4.3_A1.1_T1.js
if (typeof Promise.race !== "function") {
  throw "Promise.race should be a function";
}
if (Promise.race.length !== 1) {
  throw "Promise.race.length should be 1";
}
if (Promise.propertyIsEnumerable("race")) {
  throw "Promise.race should not be enumerable";
}
