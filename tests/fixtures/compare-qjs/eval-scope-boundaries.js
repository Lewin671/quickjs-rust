(() => {
  function late(declaration, generator) {
    function inner() { eval(''); return () => b; }
    function* suspended() { eval(''); yield; return b; }
    const g = generator ? suspended() : inner();
    if (generator) g.next();
    eval(declaration);
    const value = generator ? g.next().value : g();
    return typeof value === 'function' ? value() : value;
  }
  function a() { var x = 1; return function b() {
    eval(''); function c() { eval(''); return () => x; }
    var f = c(); eval('var x = 2'); return [f(), x].join(',');
  }; }
  function constant() {
    const t = 3; const f = function() { eval('t = 4'); };
    let threw = false;
    try { f(); } catch (e) { threw = e instanceof TypeError; } return t === 3 && threw;
  }
  function tdz() {
    const f = function() { eval('t = 4'); };
    let threw = false; try { f(); } catch (e) { threw = e instanceof ReferenceError; }
    let result; try { t; result = false; } catch(e) { result = e instanceof ReferenceError; }
    let t = 3; return result && threw;
  }
  class D { #p; [(#p in {}, this === globalThis ? 'ok' : 'bad')]() {} }
  return [late('var b = 5', false), late('function b(){return 5}', false),
    late('var b = 5', true), late('function b(){return 5}', true),
    a()(), constant(), tdz(), typeof new D().ok].join(':');
})()
