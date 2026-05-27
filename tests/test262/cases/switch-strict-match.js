// Derived from: test/language/statements/switch/S12.11_A1_T3.js
var value = "1";
var result = 0;

switch (value) {
  case 1:
    result = 1;
    break;
  default:
    result = 2;
}

if (result !== 2) { throw; }
