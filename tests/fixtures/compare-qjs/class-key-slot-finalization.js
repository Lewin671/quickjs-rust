(() => {
  function f(a, ...rest) {
    try {
      let key = 'x'; let {[key]: x = 2, ...other} = a;
      with (other) { x = x + y; }
      class C { #p; #value = x;
        [(#p in {}, eval('arguments[1]'))]() { return x + other.y; }
        get value() { return this.#value; } copy = rest[1]; }
      let c = new C(); return [c.m(), c.value, c.copy, x, rest[0]].join(':');
    } catch (error) { return error.name; }
  }
  function caught(name) {
    try { class C { #p; [(#p in {}, eval('arguments[0]'))]() {} } throw 3; }
    catch (error) { return error + arguments[0]; }
  }
  function* first(name) {
    let read; for (let i=0;i<2;i++) { read=()=>i+name; }
    class C { #p; [(#p in {}, eval('arguments[0]'))]() {} }
    yield read(); return typeof new C().m;
  }
  function* second(name) {
    yield* [1,2]; class C { #p; [(#p in {}, eval('arguments[0]'))]() {} }
    return typeof new C().m;
  }
  function assigned() {
    arguments=['m']; class C { #p; [(#p in {}, arguments[0])]() {} }
    return typeof new C().m+':'+arguments[0];
  }
  let read;
  function escaped() {
    try { read=()=>arguments[0]; class C { #p; [(#p in {}, 'm')]() {} } throw 0; }
    catch(error) {}
    return arguments[0];
  }
  let afterCatch=escaped('m');
  function live() {
    function inner() {
      eval("class C { #p; [(#p in {}, arguments[0])]() {} }");
      return function() { return later; };
    }
    let get=inner('m'); eval('var later=5'); return get();
  }
  function reassigned(value) {
    let read=eval("let read; class C { #p; [(#p in {}, read=()=>arguments[0], 'm')]() {} } read;");
    arguments=['changed']; return read();
  }
  let a=first('m'), b=second('m');
  return [f({x:2,y:3},'m','tail'),caught('m'),a.next().value,a.next().value,
    b.next().value,b.next().value,b.next().value,assigned('old'),read(),afterCatch,
    live(),reassigned('m')].join('|');
})()
