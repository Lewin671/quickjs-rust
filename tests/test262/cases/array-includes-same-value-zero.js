// Derived from: test/built-ins/Array/prototype/includes/samevaluezero.js
if (![-0].includes(0)) { throw; }
if (![(0 / 0)].includes(0 / 0)) { throw; }
if ([42].includes("42")) { throw; }
