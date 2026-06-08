// Derived from: test/built-ins/RegExp/from-regexp-like.js
var obj = {
  source: "source text",
  flags: "i"
};

obj[Symbol.match] = [];
var result = new RegExp(obj);

if (Object.getPrototypeOf(result) !== RegExp.prototype) {
  throw "RegExp regexp-like construction should use RegExp.prototype";
}

if (result.source !== "source text") {
  throw "RegExp regexp-like construction should read source";
}

if (result.flags !== "i") {
  throw "RegExp regexp-like construction should read flags";
}
