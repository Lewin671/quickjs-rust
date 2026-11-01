// Sloppy and strict compound/update stores to a configurable own global
// property, read at their own start, through the exact global store.
globalThis.C = 0;
function bump() { C += 2; C++; }
function bumpStrict() { "use strict"; C -= 1; }
for (var i = 0; i < 500000; i++) { bump(); bumpStrict(); }
if (C !== 1000000) throw new Error("bad " + C);
