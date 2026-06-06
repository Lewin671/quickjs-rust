// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-430.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-439.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-448.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-457.js
function assertConfigurableAccessorCanBecomeData(enumerable) {
  let object = {};
  Object.defineProperty(object, "prop", {
    get: undefined,
    set: undefined,
    enumerable: enumerable,
    configurable: true,
  });
  let before = Object.getOwnPropertyDescriptor(object, "prop");
  Object.defineProperty(object, "prop", { value: 1001 });
  let after = Object.getOwnPropertyDescriptor(object, "prop");
  if (!before.hasOwnProperty("get") || !after.hasOwnProperty("value")) {
    throw "expected configurable undefined accessor to become data property";
  }
}

function assertNonConfigurableAccessorRejectsData(enumerable) {
  let object = {};
  Object.defineProperty(object, "prop", {
    get: undefined,
    set: undefined,
    enumerable: enumerable,
    configurable: false,
  });
  let caught = false;
  try {
    Object.defineProperty(object, "prop", { value: 1001 });
  } catch (error) {
    caught = error instanceof TypeError;
  }
  let after = Object.getOwnPropertyDescriptor(object, "prop");
  if (!caught || !after.hasOwnProperty("get") || after.hasOwnProperty("value")) {
    throw "expected non-configurable undefined accessor to reject data property";
  }
}

assertConfigurableAccessorCanBecomeData(true);
assertConfigurableAccessorCanBecomeData(false);
assertNonConfigurableAccessorRejectsData(true);
assertNonConfigurableAccessorRejectsData(false);
