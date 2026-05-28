(function () {
  function add(a, b, c) { return this.base + a + b + (c || 0); }
  function count() { return arguments.length; }
  function forward() { return add.apply({ base: 1 }, arguments); }
  let context = { base: 4 };
  return add.apply(context, [2, 3, 0]) + ":" + count.apply(null, undefined) + ":" + forward(2, 3, 4) + ":" + add.apply.length;
})()
