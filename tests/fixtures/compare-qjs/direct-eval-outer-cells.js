(() => {
  function outer(value) {
    const read = function () { return eval('value'); };
    const write = function () { return eval('++value'); };
    value = 9;
    return [read, write, () => value];
  }
  function middle(value) {
    return function () { return () => () => eval('++value'); };
  }
  function scopes(value) {
    let read;
    { let value = 11; read = function () { return eval('value'); }; }
    let list = [];
    for (let i = 0; i < 3; i++) {
      list.push(function () { return eval('i'); });
    }
    return read() + ':' + list.map(f => f()).join(',');
  }
  function delayed() {
    const fixed = 3;
    let read = function () { return eval('later'); };
    let tdz;
    try { read(); } catch (e) { tdz = e instanceof ReferenceError; }
    let later = 4;
    const own = function (value) { return eval('value'); };
    const readFixed = function () { return eval('fixed'); };
    return tdz + ':' + read() + ':' + own(6) + ':' + readFixed();
  }
  function members(value) {
    return [class { method() { return eval('value'); } },
      function (x = eval('value')) { return x; }];
  }
  function block(value) {
    return function () {
      { let value = 1; }
      try { throw 2; } catch (value) { value; }
      return eval('value');
    };
  }
  function caught(value) {
    try {
      let arguments = 'bad';
      class D { #k; [(#k in {}, arguments[0])]() {} }
      throw 0;
    } catch (error) {}
    return eval('arguments[0]');
  }
  const c = outer(8), d = middle(8)()(), m = members(8);
  return [c[0](), c[1](), c[2](), d(), d(), scopes(8), delayed(),
    new m[0]().method(), m[1](), block(8)(), caught('ok')].join(':');
})()
