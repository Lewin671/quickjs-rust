(function () {
  function check(value, label) { if (!value) throw Error(label); }
  function read(o) { return o.x; }
  var base = { x: 1 }, middle = Object.create(base), o = Object.create(middle);
  for (var i = 0; i < 30; i++) check(read(o) === 1, "inherit");
  base.x = 2;
  check(read(o) === 2, "live value");
  middle.x = 3;
  check(read(o) === 3, "shadow");
  delete middle.x;
  check(read(o) === 2, "delete shadow");
  Object.setPrototypeOf(middle, { x: 4 });
  check(read(o) === 4, "middle edge");
  Object.setPrototypeOf(o, { x: 5 });
  check(read(o) === 5, "first edge");
  var calls = 0, receiver;
  Object.defineProperty(base, "x", { set: function (v) { calls++; receiver = this; }, configurable: true });
  var a = [];
  check(Reflect.set(base, "x", 7, a) && receiver === a && calls === 1, "Reflect receiver");
  class Parent { set x(v) { calls++; receiver = this; } }
  class Child extends Parent { write(v) { super.x = v; } }
  var child = new Child();
  child.write(8);
  check(receiver === child && calls === 2, "super receiver");
  var traps = 0;
  var proxy = new Proxy({ x: 9 }, { get: function (t, k, r) { traps++; return Reflect.get(t, k, r); } });
  check(read(proxy) === 9 && read(proxy) === 9 && traps === 2, "Proxy effects");
  function f(v) {}
  function g(v) {}
  f.x = 11; g.x = 12;
  check(read(f) === 11 && read(g) === 12, "function own slots");
  function prototype(fn) { return fn.prototype; }
  check(prototype(f).constructor === f && prototype(g).constructor === g, "lazy prototypes");
  check(Object.getOwnPropertyNames(f).join() === "length,name,prototype,x", "function order");
  var symbol = Symbol();
  a.z = 1; a.y = 2; a[symbol] = 3;
  delete a.z; a.z = 4;
  check(Object.keys(a).join() === "y,z" && a[symbol] === 3, "array order and symbol");
  Object.freeze(a);
  check(!Reflect.set(a, "z", 5) && !Reflect.set(a, symbol, 6), "array freeze");
  var buffer = new ArrayBuffer(8, { maxByteLength: 16 });
  var typed = new Uint8Array(buffer); typed.x = 13; typed[0] = 7;
  check(read(typed) === 13, "typed named");
  buffer.resize(0);
  check(read(typed) === 13 && typed[0] === undefined, "resize");
  buffer.resize(16);
  check(typed.length === 16 && typed[0] === 0, "regrow");
  buffer.transfer();
  check(read(typed) === 13 && typed[0] === undefined && typed.length === 0, "detach");
  return true;
})()
