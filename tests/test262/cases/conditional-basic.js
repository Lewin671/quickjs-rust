// Derived from: test/language/expressions/conditional/S11.12_A2.1_T1.js
if ((true ? false : true) !== false) {
  throw;
}

if ((false ? false : true) !== true) {
  throw;
}
