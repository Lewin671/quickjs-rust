// Derived from: test/built-ins/Object/keys/15.2.3.14-3-4.js
function keys() {
  return Object.keys(arguments).join("|");
}

if (keys(1, 2, 3) !== "0|1|2") {
  throw "Object.keys(arguments) should enumerate argument indices only";
}
