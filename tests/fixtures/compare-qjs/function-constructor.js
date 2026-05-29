(function () {
  let add = Function("a", "b", "return a + b;");
  let pair = new Function("a,b", "return a + ':' + b;");
  let C = Function("x", "this.x = x;");
  let instance = new C(9);
  return typeof add + ":" + add.length + ":" + add(2, 3) + ":" + pair(4, 5) + ":" + instance.x + ":" + (Object.getPrototypeOf(add) === Function.prototype);
})()
