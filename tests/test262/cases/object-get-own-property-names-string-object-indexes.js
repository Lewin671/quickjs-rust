// Derived from: test/built-ins/Object/getOwnPropertyNames/15.2.3.4-4-44.js
var str = new String("abc");
str[5] = "de";
var names = Object.getOwnPropertyNames(str);
if (names.length !== 5) { throw; }
if (names[0] !== "0") { throw; }
if (names[1] !== "1") { throw; }
if (names[2] !== "2") { throw; }
if (names[3] !== "5") { throw; }
if (names[4] !== "length") { throw; }
