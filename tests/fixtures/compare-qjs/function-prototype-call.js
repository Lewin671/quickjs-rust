(function () {
  function add(a, b) { return this.base + a + b; }
  function getThis() { return this; }
  let context = { base: 4 };
  return add.call(context, 2, 3) + ":" + (getThis.call(undefined) === this) + ":" + (Object.getPrototypeOf(add) === Function.prototype) + ":" + Function.prototype.isPrototypeOf(add) + ":" + add.call.length;
})()
