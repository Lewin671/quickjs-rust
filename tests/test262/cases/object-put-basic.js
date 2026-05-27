// Derived from: test/language/expressions/assignment/S8.12.5_A2.js
var map = { 1: "one", two: 2 };
map[1] = "uno";
if (map[1] !== "uno") { throw; }
map["1"] = 1;
if (map[1] !== 1) { throw; }
map["two"] = "two";
if (map["two"] !== "two") { throw; }
map.two = "duo";
if (map.two !== "duo") { throw; }
