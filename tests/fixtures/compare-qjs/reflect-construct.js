(function () {
  function C(a, b) {
    this.sum = a + b;
    this.proto = Object.getPrototypeOf(this);
  }

  var args = { 0: 3, 1: 4, length: 2 };
  var basic = Reflect.construct(C, [1, 2]);
  var withNewTarget = Reflect.construct(C, args, Array);
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

  return Reflect.construct.length + ":" +
    basic.sum + ":" +
    (basic instanceof C) + ":" +
    withNewTarget.sum + ":" +
    (Object.getPrototypeOf(withNewTarget) === Array.prototype) + ":" +
    (withNewTarget.proto === Array.prototype) + ":" +
    targetError + ":" +
    argsError + ":" +
    newTargetError;
})()
