// Derived from: test/built-ins/Array/S15.4_A1.1_T4.js
let keys = ["01", "1e0", "+1", "-0", " 1", "4294967295"];
let array = [];
for (let key of keys) {
  array[key] = key;
}
if (array.length !== 0 || Object.keys(array).length !== keys.length) {
  throw "non-canonical numeric strings must not create array elements";
}
for (let key of keys) {
  if (!Object.prototype.hasOwnProperty.call(array, key) || array[key] !== key) {
    throw "non-canonical numeric string must retain its exact property identity";
  }
}

array["1"] = "indexed";
if (array.length !== 2 || array[1] !== "indexed") {
  throw "canonical numeric string must still create an array element";
}
array.length = 0;
if (array[1] !== undefined || array["01"] !== "01" || array["4294967295"] !== "4294967295") {
  throw "length truncation must remove indices but preserve ordinary string properties";
}

let deleted = [];
deleted[1] = "indexed";
deleted["01"] = "named";
delete deleted["01"];
if (deleted[1] !== "indexed" || Object.prototype.hasOwnProperty.call(deleted, "01")) {
  throw "delete must distinguish a non-canonical key from its numeric spelling";
}

let reflected = [];
reflected[1] = "indexed";
reflected["01"] = "named";
if (!Reflect.deleteProperty(reflected, "01") || !Reflect.deleteProperty(reflected, "1")) {
  throw "Reflect.deleteProperty must report successful configurable deletions";
}
if (reflected.length !== 2
    || Object.prototype.hasOwnProperty.call(reflected, "01")
    || Object.prototype.hasOwnProperty.call(reflected, "1")) {
  throw "Reflect.deleteProperty must delete the exact named or indexed property";
}

let defined = [];
Object.defineProperty(defined, "01", { value: 5, configurable: true });
if (defined.length !== 0 || defined["01"] !== 5) {
  throw "defineProperty must not grow length for a non-canonical numeric string";
}
