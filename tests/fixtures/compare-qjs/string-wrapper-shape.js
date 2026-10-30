(() => {
  class S extends String {
    #value = 7;
    read() { return this.#value; }
  }
  const s = new S('a😀');
  const descriptor = Object.getOwnPropertyDescriptor(s, 'length');
  const seen = [];
  const proxy = new Proxy(s, {
    get(target, key, receiver) {
      seen.push(String(key));
      return Reflect.get(target, key, receiver);
    }
  });
  const length = proxy.length;
  const index = proxy[0];
  Object.freeze(s);
  return [s.length, descriptor.writable, descriptor.enumerable,
    descriptor.configurable, Object.keys(s).join(','), s.read(), String(s),
    Reflect.defineProperty(s, 'length', { value: 4 }), length, index,
    seen.join(',')].join('|');
})()
