(() => {
  const a = Array.prototype;
  let invalid = false;
  function write(v) { a.length = v; }
  for (let i = 0; i < 40; i++) write(0);
  try { write(-1); } catch (e) { invalid = e instanceof RangeError; }
  const receiver = {};
  const receiverSet = Reflect.set(a, 'length', -1, receiver);
  Object.defineProperty(a, '2', {value: 2, configurable: false});
  Object.defineProperty(a, '4', {value: 4, configurable: true});
  const extended = a.length;
  const shrunk = Reflect.defineProperty(a, 'length', {value: 1, writable: false});
  const blocked = !Reflect.set(a, '5', 5);
  let calls = 0;
  const value = {valueOf() { calls++; return 1; }};
  const setReadonly = Reflect.set(a, 'length', 1);
  const before = calls;
  const defineReadonly = Reflect.defineProperty(a, 'length', {value: value});
  return [invalid, receiverSet, receiver.length, extended, shrunk, a.length,
    a.hasOwnProperty('4'), a.hasOwnProperty('2'), blocked,
    setReadonly, before, defineReadonly, calls,
    Object.getOwnPropertyDescriptor(a, 'length').writable].join(':');
})()
