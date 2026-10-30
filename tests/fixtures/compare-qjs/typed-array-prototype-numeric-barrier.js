(function () {
  var base = Object.create(Uint8Array.prototype);
  ['-0', 'Infinity', '2', '1.5', 'foo'].forEach(function (k) { base[k] = 'inherited'; });
  var ta = new Uint8Array([7]); Object.setPrototypeOf(ta, base);
  var o = Object.create(ta), result = [];
  function read(o, key) { return o[key]; }
  ['0', '-0', 'Infinity', '2', '1.5', 'foo'].forEach(function (k) {
    for (var i = 0; i < 100; i++) read(o, k);
    result.push(String(read(o, k)), k in o, Reflect.has(o, k), String(Reflect.get(o, k)));
  });
  return result.join('|');
})()
