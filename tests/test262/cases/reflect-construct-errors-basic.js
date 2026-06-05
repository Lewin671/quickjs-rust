// Derived from: test/built-ins/Reflect/construct/target-is-not-constructor-throws.js
function C() {}

var targetError = false;
var argsError = false;
var newTargetError = false;

try {
  Reflect.construct(1, []);
} catch (error) {
  targetError = error.constructor === TypeError;
}

try {
  Reflect.construct(C, 1);
} catch (error) {
  argsError = error.constructor === TypeError;
}

try {
  Reflect.construct(C, [], Reflect.apply);
} catch (error) {
  newTargetError = error.constructor === TypeError;
}

if (!targetError || !argsError || !newTargetError) {
  throw "Reflect.construct should reject invalid target, argumentsList, and newTarget";
}
