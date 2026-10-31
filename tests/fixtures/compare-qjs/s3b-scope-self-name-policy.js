(() => {
  let f = 'before'; const run = function f(callback) { callback(); return typeof f; };
  const callback = run(() => f = 'after') + ':' + f;
  const owner = function self() { self = 8; eval('self = 9'); return typeof self; };
  const shadow = function self(self) { self = 8; return self; };
  return [callback, owner(), shadow(1)].join('|');
})()
