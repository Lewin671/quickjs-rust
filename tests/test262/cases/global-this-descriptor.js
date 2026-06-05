// Derived from: test/staging/sm/global/globalThis-enumeration.js
var descriptor = Object.getOwnPropertyDescriptor(this, "globalThis");
if (descriptor.value !== this) {
  throw new Error("globalThis descriptor value must be the global object");
}
if (!descriptor.writable) {
  throw new Error("globalThis must be writable");
}
if (descriptor.enumerable) {
  throw new Error("globalThis must not be enumerable");
}
if (!descriptor.configurable) {
  throw new Error("globalThis must be configurable");
}
