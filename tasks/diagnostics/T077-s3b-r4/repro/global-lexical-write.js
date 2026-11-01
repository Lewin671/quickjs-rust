// Sloppy and strict callees write a script lexical: the declarative record
// takes every store.
let L = 0;
function put(x) { L = x; }
function putStrict(x) { "use strict"; L += x; }
for (var i = 0; i < 500000; i++) { put(i); putStrict(1); }
if (L !== 500000) throw new Error("bad " + L);
