// Derived from: test/built-ins/Object/values/inherited-properties-omitted.js
var object = Object.create({ inherited: 1 }, { own: { value: 2, enumerable: true } });
var values = Object.values(object);
if (values.length !== 1 || values[0] !== 2) {
  throw "Object.values should omit inherited properties";
}
