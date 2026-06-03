// Derived from: test/built-ins/RegExp/prototype/source/value-slash.js
var re = eval("/" + new RegExp("/").source + "/");

if (re.test("/") !== true) {
  throw "escaped slash source should match slash";
}

if (re.test("_/_") !== true) {
  throw "escaped slash source should match slash inside input";
}

if (re.test("\\") !== false) {
  throw "escaped slash source should not match backslash";
}
