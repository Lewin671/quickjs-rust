// Derived from: test/language/statements/switch/cptn-dflt-b-fall-thru-nrml.js
var value = "missing";
var result = 0;

switch (value) {
  default:
    result += 1;
  case "next":
    result += 2;
}

if (result !== 3) { throw; }
