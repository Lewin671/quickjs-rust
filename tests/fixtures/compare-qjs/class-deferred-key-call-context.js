(() => {
  class Outer {
    static #staticKey = "method";
    #key = "method";
    static method() {
      class D { [this.#staticKey]() { return 1; } }
      return new D().method();
    }
    method() {
      return () => () => {
        class D { [(() => this.#key)()]() { return 2; } }
        return new D().method();
      };
    }
    default(C = class { [this.#key] = 3; }) { return new C().method; }
    field = (() => {
      class D { get [this.#key]() { return 4; } }
      return new D().method;
    })();
    static field = (() => {
      class D { [this.#staticKey]() { return 5; } }
      return new D().method();
    })();
    static {
      this.field += (() => {
        class D { [this.#staticKey] = 6; }
        return new D().method;
      })();
    }
  }
  const outer = new Outer();
  const read = outer.method();
  return [Outer.method(), read.call({}).call({}), outer.default(),
          outer.field, Outer.field].join(":");
})()
