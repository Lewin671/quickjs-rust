// Derived from: test/language/types/reference/S8.7_A7.js
var n = {};
var m = n;
function populateAge(person) { person.age = 50; }
populateAge(m);
if (n.age !== 50) { throw; }
