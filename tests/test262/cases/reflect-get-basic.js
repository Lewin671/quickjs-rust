// Derived from: test/built-ins/Reflect/get/return-value.js
var object = Object.create({ inherited: 2 });
object.value = 1;
if (Reflect.get(object, "value") !== 1) {
  throw "expected Reflect.get to return own data property";
}
if (Reflect.get(object, "inherited") !== 2) {
  throw "expected Reflect.get to follow the prototype chain";
}
if (Reflect.get(object, "missing") !== undefined) {
  throw "expected Reflect.get to return undefined for missing properties";
}
