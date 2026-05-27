// Derived from: test/language/statements/block/S12.1_A2.js
try {
  throw "catchme";
  throw "unreachable";
} catch (error) {
  if (error !== "catchme") { throw; }
}
