// Derived from: test/built-ins/RegExp/from-regexp-like-flag-override.js
var obj = {
  source: "source text"
};

Object.defineProperty(obj, "flags", {
  get: function() {
    throw "flags should not be read when flags are provided";
  }
});

obj[Symbol.match] = true;
var result = new RegExp(obj, "g");

if (result.source !== "source text") {
  throw "RegExp regexp-like construction should read source with flag override";
}

if (result.flags !== "g") {
  throw "RegExp regexp-like construction should use provided flags";
}
