// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-10.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-11.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-12.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-13.js

var getter = function() {
  return 1;
};

var enumerableTarget = {};
Object.defineProperty(enumerableTarget, "prop", {
  get: getter,
  enumerable: false,
  configurable: false
});
var enumerableThrew = false;
try {
  Object.defineProperty(enumerableTarget, "prop", {
    get: getter,
    enumerable: true
  });
} catch (error) {
  enumerableThrew = true;
}
if (!enumerableThrew) {
  throw "expected enumerable change to throw";
}
var enumerableDesc = Object.getOwnPropertyDescriptor(enumerableTarget, "prop");
if (enumerableDesc.get !== getter) {
  throw "expected getter to remain unchanged";
}
if (enumerableDesc.enumerable !== false) {
  throw "expected enumerable to remain false";
}

var dataTarget = {};
Object.defineProperty(dataTarget, "prop", {
  value: 101,
  configurable: false
});
var dataKindThrew = false;
try {
  Object.defineProperty(dataTarget, "prop", {
    get: getter
  });
} catch (error) {
  dataKindThrew = true;
}
if (!dataKindThrew) {
  throw "expected data-to-accessor change to throw";
}
var dataDesc = Object.getOwnPropertyDescriptor(dataTarget, "prop");
if (dataDesc.value !== 101) {
  throw "expected data property value to remain unchanged";
}

var accessorTarget = {};
Object.defineProperty(accessorTarget, "prop", {
  get: getter,
  configurable: false
});
var accessorKindThrew = false;
try {
  Object.defineProperty(accessorTarget, "prop", {
    value: 101
  });
} catch (error) {
  accessorKindThrew = true;
}
if (!accessorKindThrew) {
  throw "expected accessor-to-data change to throw";
}
var accessorDesc = Object.getOwnPropertyDescriptor(accessorTarget, "prop");
if (accessorDesc.get !== getter) {
  throw "expected accessor getter to remain unchanged";
}

Object.defineProperty(accessorTarget, "prop", {});
if (Object.getOwnPropertyDescriptor(accessorTarget, "prop").get !== getter) {
  throw "expected empty descriptor to preserve accessor getter";
}
