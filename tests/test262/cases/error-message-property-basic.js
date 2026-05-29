// Derived from: test/built-ins/Error/message_property.js
var value = new Error("my-message");
var descriptor = Object.getOwnPropertyDescriptor(value, "message");
if (value.message !== "my-message") { throw; }
if (descriptor.value !== "my-message") { throw; }
if (descriptor.writable !== true) { throw; }
if (descriptor.enumerable !== false) { throw; }
if (descriptor.configurable !== true) { throw; }
