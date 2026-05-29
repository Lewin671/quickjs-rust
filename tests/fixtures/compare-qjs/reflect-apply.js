(function () {
  function add(a, b) {
    return this.base + a + b;
  }
  var context = { base: 4 };
  function count() {
    return arguments.length;
  }
  function value() {
    return 57;
  }
  return [
    Reflect.apply(add, context, [2, 3]),
    Reflect.apply(count, null, []),
    Reflect.apply(value, undefined, []),
    Reflect.apply.length
  ].join(":");
})()
