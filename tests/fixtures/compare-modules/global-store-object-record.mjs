// A global store's object record runs HasProperty over the whole prototype
// chain and [[Set]]s the global object, at script top level, in functions
// and in hot functions; a Proxy in the chain observes both traps.
const evalScript = typeof __quickjsRustEvalScript === 'function'
  ? __quickjsRustEvalScript : (await import('qjs:std')).evalScript;
evalScript(`
var log = [], r = [];
function hot(f) { for (var i = 0; i <= 50; i++) f(i, i === 50); }
Object.defineProperty(Object.prototype, "acc", { get() { return 10; }, set(v) { log.push("set:" + v); }, configurable: true });
Object.defineProperty(Object.prototype, "nw", { value: 5, writable: false, configurable: true });
function own(n) { return Object.prototype.hasOwnProperty.call(globalThis, n) ? "own=" + globalThis[n] : "none"; }
acc += 2; acc++; acc ||= 3; nw += 1;
r.push(own("acc"), own("nw"));
try { (function () { "use strict"; nw++; })(); } catch (e) { r.push(e.name); }
function mk(n) { Object.defineProperty(globalThis, n, { value: 1, writable: true, configurable: true }); }
mk("acc"); acc = (delete globalThis.acc, 4);
r.push(own("acc"), log.join(","));
var proto = Object.getPrototypeOf(globalThis), plog = [];
Object.setPrototypeOf(globalThis, new Proxy(proto, {
  has(t, key) { if (key === "pk") plog.push("has"); return Reflect.has(t, key); },
  set(t, key, v, rcv) { if (key === "pk") { plog.push("set:" + v); return true; } return Reflect.set(t, key, v, rcv); },
}));
function mkp() { Object.defineProperty(globalThis, "pk", { value: 1, writable: true, configurable: true }); }
mkp(); pk = (delete globalThis.pk, 2);
function g(i) { "use strict"; mkp(); try { pk = (delete globalThis.pk, i); return "nothrow"; } catch (e) { return e.name; } }
var last;
hot(function (i) { last = g(i); });
r.push(own("pk"), last, plog.slice(0, 4).join(","), plog.length);
`);
print(evalScript('r.join(" ")'));
