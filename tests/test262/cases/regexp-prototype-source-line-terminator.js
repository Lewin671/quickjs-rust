// Derived from: test/built-ins/RegExp/prototype/source/value-line-terminator.js
var re = eval("/" + new RegExp("\n").source + "/");

if (re.test("\n") !== true) {
  throw "escaped RegExp source should match a line feed";
}

if (re.test("_\n_") !== true) {
  throw "escaped RegExp source should match a line feed within input";
}

if (re.test("\\n") !== false) {
  throw "escaped RegExp source should not match a literal n escape sequence";
}

if (re.test("\r") !== false) {
  throw "escaped RegExp source should not match carriage return";
}

if (re.test("n") !== false) {
  throw "escaped RegExp source should not match literal n";
}
