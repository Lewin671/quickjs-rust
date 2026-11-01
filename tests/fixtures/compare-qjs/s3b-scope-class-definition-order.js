(() => {
  let effects = '', read, tdz;
  try { class Bad extends 1 { [(effects += 'bad')]() {} } } catch (e) {}
  try { class Bad { static ['prototype']() {} [(effects += 'later')]() {} } } catch (e) {}
  const C = class Inner {
    [(read = () => Inner, (() => { try { Inner; } catch(e) { tdz = e instanceof ReferenceError; } })(), 'm')]() { return Inner; }
    static a = (effects += 'a'); static #b = (effects += 'b');
    static { effects += 'c'; } static #d = (effects += 'd');
    static e = (effects += 'e');
  };
  let Display = class { m() { return Display; } }; const original = Display; Display = 8;
  return [effects, tdz, read() === C, new C().m() === C, original.name, new original().m()].join(':');
})()
