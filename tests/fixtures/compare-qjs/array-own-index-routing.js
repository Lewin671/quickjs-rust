(() => {
  const log = [];
  const prototype = new Proxy({}, {
    set(target, key) { log.push('set:' + String(key)); return true; }
  });
  const a = [10, , 30];
  Object.setPrototypeOf(a, prototype);
  a[0] = 11;
  a[1] = 12;
  a[2] = 31;
  Object.defineProperty(a, '2', { writable: false });
  a[2] = 99;
  const b = [1, 2];
  Object.setPrototypeOf(b, prototype);
  Object.preventExtensions(b);
  b[0] = 4;
  return [a[0], Object.hasOwn(a, '1'), a[2], a.length,
    b[0], Object.keys(a).join(','), log.join(',')].join('|');
})()
