// Derived from: test/language/statements/switch/S12.11_A2_T1.js
var value = 2;
var result = 0;

switch (value) {
  case 1:
    result = 1;
    break;
  case 2:
    result = 2;
    break;
  default:
    result = 3;
}

if (result !== 2) { throw; }
