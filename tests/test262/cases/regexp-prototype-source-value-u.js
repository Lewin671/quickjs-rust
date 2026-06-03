// Derived from: test/built-ins/RegExp/prototype/source/value-u.js

var re;

re = eval("/" + /\ud834\udf06/u.source + "/u");

if (re.test("\ud834\udf06") !== true) {
  throw "surrogate pair source should match the UTF-16 surrogate pair";
}

if (re.test("𝌆") !== true) {
  throw "surrogate pair source should match the astral character";
}

re = eval("/" + /\u{1d306}/u.source + "/u");

if (re.test("\ud834\udf06") !== true) {
  throw "braced unicode source should match the UTF-16 surrogate pair";
}

if (re.test("𝌆") !== true) {
  throw "braced unicode source should match the astral character";
}

if (re.test("x𝌆y") !== true) {
  throw "braced unicode source should match within input";
}

if (re.exec("x𝌆y").index !== 1) {
  throw "braced unicode source should report the character index";
}
