// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-117.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-125.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-133.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-134.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-136.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-148.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-149.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-150.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-151.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-188.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-189.js
function assertThrowsRangeError(value) {
  let array = [];
  let caught = false;
  try {
    Object.defineProperty(array, "length", { value });
  } catch (error) {
    caught = error instanceof RangeError;
  }
  if (!caught || array.length !== 0) {
    throw "expected invalid array length to throw RangeError";
  }
}

assertThrowsRangeError(undefined);
assertThrowsRangeError(-9);
assertThrowsRangeError(Infinity);
assertThrowsRangeError(NaN);

let array = [0, 1, 2];
Object.defineProperty(array, "2", { configurable: false });
let caught = false;
try {
  Object.defineProperty(array, "length", { value: 1 });
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught || array.length !== 3 || array[2] !== 2) {
  throw "expected non-configurable array element to block length shrink";
}

array = [1, 2, 3];
Object.defineProperty(array, "length", { writable: false });
caught = false;
try {
  Object.defineProperty(array, "3", { value: "abc" });
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught || array.length !== 3 || array.hasOwnProperty("3")) {
  throw "expected non-writable length to reject index equal to length";
}

caught = false;
try {
  Object.defineProperty(array, "4", { value: "abc" });
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught || array.length !== 3 || array.hasOwnProperty("4")) {
  throw "expected non-writable length to reject index greater than length";
}

array = [];
let valueOfAccessed = false;
let toStringAccessed = false;
Object.defineProperty(array, "length", {
  value: {
    valueOf: function() {
      valueOfAccessed = true;
      return {};
    },
    toString: function() {
      toStringAccessed = true;
      return "2";
    },
  },
});
if (array.length !== 2 || !valueOfAccessed || !toStringAccessed) {
  throw "expected length coercion to fall back to toString";
}

array = [];
valueOfAccessed = false;
toStringAccessed = false;
Object.defineProperty(array, "length", {
  value: {
    valueOf: function() {
      valueOfAccessed = true;
      return 3;
    },
    toString: function() {
      toStringAccessed = true;
      return "2";
    },
  },
});
if (array.length !== 3 || !valueOfAccessed || toStringAccessed) {
  throw "expected length coercion to prefer valueOf";
}

array = [];
valueOfAccessed = false;
toStringAccessed = false;
caught = false;
try {
  Object.defineProperty(array, "length", {
    value: {
      valueOf: function() {
        valueOfAccessed = true;
        return {};
      },
      toString: function() {
        toStringAccessed = true;
        return {};
      },
    },
  });
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught || array.length !== 0 || !valueOfAccessed || !toStringAccessed) {
  throw "expected non-primitive length coercion to throw";
}

array = [];
let proto = {
  valueOf: function() {
    valueOfAccessed = true;
    return 4;
  },
};
let lengthObject = Object.create(proto);
lengthObject.toString = function() {
  toStringAccessed = true;
  return "2";
};
valueOfAccessed = false;
toStringAccessed = false;
Object.defineProperty(array, "length", { value: lengthObject });
if (array.length !== 4 || !valueOfAccessed || toStringAccessed) {
  throw "expected inherited valueOf to coerce length";
}
