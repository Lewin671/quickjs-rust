// Derived from: test/built-ins/String/prototype/includes/String.prototype.includes_Success.js
if (!"test262".includes("262")) {
  throw "expected includes to find substring";
}
if ("test262".includes("262", 5)) {
  throw "expected includes to honor start position";
}
