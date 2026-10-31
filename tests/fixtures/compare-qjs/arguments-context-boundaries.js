[
  eval("class C { #p; [(#p in {}, eval('1'))]() {} } typeof arguments;"),
  (() => {
    class C { #p; [(#p in {}, eval('1'))]() {} }
    return typeof arguments;
  })(),
  (new (class Outer {
    static { class C { #p; [(#p in {}, eval('1'))]() {} } }
    value = class C { #p; [(#p in {}, eval('1'))]() {} };
  })(), typeof arguments),
  (function owner(value) {
    return () => () => {
      class C { #p; [(#p in {}, eval('1'))]() {} }
      return arguments[0];
    };
  })(8)()(),
  (function evalOwner(value) {
    return eval("class C { #p; [(#p in {}, arguments[0])]() {} } typeof new C().method;");
  })('method'),
  (function escaping(value) {
    const read = eval("let read; class C { #p; [(#p in {}, read = () => arguments[0], 'm')]() {} } read;");
    eval("arguments = ['changed']");
    return read();
  })('method')
].join(':')
