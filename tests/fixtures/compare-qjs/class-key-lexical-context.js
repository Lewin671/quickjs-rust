(() => {
  let targetRead, privateRead;
  class Parent {
    static get key() { return this.name; }
    get key() { return this.keyName; }
  }
  class Outer extends Parent {
    static #s = 0;
    #k = 'm';
    constructor() {
      super();
      this.ok = new (class {
        [(Outer.#s, new.target === Outer ? 'ok' : 'bad')]() {}
        [(targetRead = () => new.target, Outer.#s, 'other')]() {}
      })();
    }
    f(value) {
      return () => {
        class D { [(this.#k, arguments[0])]() { return 9; } }
        return new D().method();
      };
    }
    static f() {
      class D { [(Outer.#s, super.key)]() { return 17; } }
      return new D().Outer();
    }
    instance() {
      this.keyName = 'm';
      class D { get [(this.#k, super.key)]() { return 18; } }
      return new D().m;
    }
    static private() {
      class D { #value = 23;
        [(Outer.#s, privateRead = obj => obj.#value, 'm')]() {} }
      return new D();
    }
    static fromEval(value) {
      class D { [(Outer.#s, eval('arguments[0] + super.key'))] = value; }
      return new D().aOuter;
    }
  }
  const outer = new Outer();
  const read = outer.f('method');
  const obj = Outer.private();
  function shadow(arguments) {
    const get = () => {
      class D { #k; [(#k in {}, arguments)] = 4; }
      return new D().changed;
    };
    arguments = 'changed'; return get;
  }
  let before, receiverRead;
  class Derived extends Parent {
    static #s = 0;
    constructor() {
      try { class D { [(Derived.#s, this.key)]() {} } }
      catch (e) { before = e instanceof ReferenceError; }
      super();
      class D { [(Derived.#s, receiverRead = () => this, 'm')]() {} }
    }
  }
  const derived = new Derived();
  return [read.call({}), typeof outer.ok.ok, targetRead() === Outer,
    Outer.f(), outer.instance(), privateRead(obj), Outer.fromEval('a'),
    shadow('old')(), before, receiverRead() === derived].join(':');
})()
