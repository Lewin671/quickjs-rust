// Derived from: test/staging/sm/global/globalThis-enumeration.js
if (globalThis !== this) {
  throw new Error("globalThis must reference the global object");
}
if (globalThis.Object !== Object) {
  throw new Error("globalThis must expose global properties");
}
