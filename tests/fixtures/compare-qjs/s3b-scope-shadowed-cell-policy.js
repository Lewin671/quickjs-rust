// The selected parameter cell owns its write policy, regardless of a root homonym.
const x = 1;
function outer(x) { return () => ++x; }
const write = outer(8);
[write(), write(), x].join(":");
