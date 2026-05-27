// Derived from: test/language/expressions/conditional/S11.12_A2.1_T1.js
if ((true ? 1 : missing) !== 1) {
  throw;
}

if ((false ? missing : 2) !== 2) {
  throw;
}
