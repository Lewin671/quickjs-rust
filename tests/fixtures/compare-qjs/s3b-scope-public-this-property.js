(() => {
  globalThis.this = 8;
  const read = () => this;
  return [(0, eval)('this === globalThis'), read() === globalThis].join(':');
})()
