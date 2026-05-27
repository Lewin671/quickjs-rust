// Derived from: test/built-ins/String/prototype/toString/string-primitive.js
if ("abc".toString() !== "abc") {
  throw "expected primitive string toString to return the receiver";
}
if ("abc".valueOf() !== "abc") {
  throw "expected primitive string valueOf to return the receiver";
}
