// Derived from: test/built-ins/Reflect/ownKeys/return-non-enumerable-keys.js
var object = {};
Object.defineProperty(object, "hidden", { value: 1 });
object.shown = 2;
var keys = Reflect.ownKeys(object);
if (keys.length !== 2) {
  throw "expected enumerable and non-enumerable own keys";
}
if (keys[0] !== "hidden" || keys[1] !== "shown") {
  throw "expected non-enumerable key to be returned";
}
