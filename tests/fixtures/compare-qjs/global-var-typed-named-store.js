(function () {
  globalThis.__typedStore = 0;
  (0, eval)('var __typedStore = 0;');
  function write(o, n) { for (var i = 0; i < n; i++) o.__typedStore = i; }
  write(globalThis, 3);
  var first = __typedStore;
  write(globalThis, 5);
  var result = [first, __typedStore, globalThis.__typedStore].join('|');
  delete globalThis.__typedStore;
  return result;
})()
