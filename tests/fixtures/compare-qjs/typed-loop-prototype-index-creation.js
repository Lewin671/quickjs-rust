(function () {
  /* A typed loop creates "3" on Object.prototype mid-entry; later hole reads
     must see it. `interp` stays in the interpreter frame (try), `wide` runs
     from the wide tier. */
  function interp(a, os, k, n) {
    try {} catch (e) {}
    var t = 0, p;
    for (var i = 0; i < n; i++) { t = t + (a[3] | 0); p = os[i]; p[k] = 9; }
    return t;
  }
  function wide(a, os, k, n) {
    var t = 0, p;
    for (var i = 0; i < n; i++) { t = t + (a[3] | 0); p = os[i]; p[k] = 9; }
    return t;
  }
  var plain = [{}, {}, {}, {}, {}, {}, {}, {}], warm = 0;
  for (var j = 0; j < 50; j++) warm += interp(new Array(5), plain, 'x', 8) + wide(new Array(5), plain, 'x', 8);
  var out = [warm];
  [interp, wide].forEach(function (f) {
    out.push(f(new Array(5), [{}, {}, {}, Object.prototype, {}, {}, {}, {}], '3', 8));
    delete Object.prototype[3];
    var mid = { y: 0 }, child = Object.create(mid), arr = new Array(5);
    Object.setPrototypeOf(arr, child);
    out.push(f(arr, [{}, {}, mid, {}, {}, {}, {}, {}], '3', 8));
  });
  return out.join(':') + ':' + new Array(5)[3];
})()
