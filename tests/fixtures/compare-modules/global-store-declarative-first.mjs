// Global stores consult the declarative record at store time: lexicals that
// another script declares (also from an assignment's RHS) and script lexicals
// written by strict code that eval or Function created.
const evalScript = typeof __quickjsRustEvalScript === 'function'
  ? __quickjsRustEvalScript : (await import('qjs:std')).evalScript;
globalThis.ES = evalScript;
evalScript(`
var r = [];
function hot(f) { for (var i = 0; i <= 50; i++) f(i, i === 50); }
globalThis.sg = 5;
(function () { "use strict"; sg = (ES("let sg = 13;"), 34); })();
r.push(globalThis.sg + ":" + (0, eval)("sg"));
globalThis.sh = 5;
hot(function (i, last) { "use strict"; sh = (last ? ES("let sh = 0;") : 0, i); });
r.push(globalThis.sh + ":" + (0, eval)("sh"));
globalThis.gq = 5;
function g(v) { "use strict"; gq = v; }
hot(g); ES("let gq = 13;"); g(7);
r.push(globalThis.gq + ":" + (0, eval)("gq"));
`);
evalScript(`
let z = 4;
var f = (0, eval)('"use strict"; (() => { z = 7; return z; })');
var seen = f();
r.push(z + ":" + seen);
eval('"use strict"; (function () { z = 9; })()');
r.push(z);
let x = 0;
var inc = eval('"use strict"; () => { x++; }');
inc(); inc(); inc();
(0, eval)('"use strict"; (function () { x += 10; })()');
r.push(x);
let y = 4;
new Function('"use strict"; y = 7;')();
r.push(y);
let w = 0;
hot((0, eval)('"use strict"; (function (i) { w = i; })'));
let v = 0;
hot(new Function("i", '"use strict"; v += i;'));
r.push(w + ":" + v);
globalThis.s = "p";
ES('let s = "L";');
function app() { s += "x"; }
app(); app();
hot(function () { s += "y"; });
r.push(globalThis.s + ":" + s.length);
`);
print(evalScript('r.join(" ")'));
