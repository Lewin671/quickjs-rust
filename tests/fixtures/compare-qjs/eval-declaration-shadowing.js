(() => {
  const out = [];
  for (const strict of [false, true]) {
    for (const declaration of ['var x = 5; x', 'function x() { return 5; } x()']) {
      for (const preRead of [false, true]) {
        let inner;
        function makeCase(x) {
          if (strict) {
            inner = function () {
              'use strict';
              const before = preRead ? eval('x') : 1;
              const inside = eval(declaration);
              const value = eval('x');
              const after = typeof value === 'function' ? value() : value;
              return before + ':' + inside + ':' + after;
            };
          } else {
            inner = function () {
              const before = preRead ? eval('x') : 1;
              const inside = eval(declaration);
              const value = eval('x');
              const after = typeof value === 'function' ? value() : value;
              return before + ':' + inside + ':' + after;
            };
          }
          return () => x;
        }
        const read = makeCase(1);
        out.push(inner() + ':' + read());
      }
    }
  }
  function outer(x) {
    return [function () {
      const get = () => x;
      eval('var x = 5');
      const child = function () { eval('var x = 7'); return eval('x'); };
      return get() + ':' + child() + ':' + x;
    }, () => x];
  }
  const c = outer(1);
  out.push(c[0]() + ':' + c[1]());
  function suspended(x) {
    return [function* () {
      const get = () => x;
      yield eval('var x = 5; x'); yield get();
    }, () => x];
  }
  const s = suspended(1), it = s[0]();
  out.push(it.next().value + ':' + it.next().value + ':' + s[1]());
  return out.join('|');
})()
