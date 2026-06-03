// Derived from: test/built-ins/RegExp/prototype/source/this-val-regexp-prototype.js
var get = Object.getOwnPropertyDescriptor(RegExp.prototype, "source").get;

if (get.call(RegExp.prototype) !== "(?:)") {
  throw "RegExp.prototype.source getter should special-case RegExp.prototype";
}

var caught = false;
try {
  get.call({});
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "RegExp.prototype.source getter should reject ordinary objects";
}

if (new RegExp("").source !== "(?:)") {
  throw "empty RegExp source should be reusable";
}

if (new RegExp("/").source !== "\\/") {
  throw "slash RegExp source should be escaped";
}
