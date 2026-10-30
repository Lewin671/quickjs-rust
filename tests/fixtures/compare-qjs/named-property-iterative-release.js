(function () {
  var head = null, middle;
  for (var i = 0; i < 200000; i++) { var a = []; a.next = head; head = a; if (i === 100000) middle = a; }
  head = a = null;
  var result = middle.next !== null;
  middle = null;
  for (var i = 0; i < 200000; i++) { var f = function () {}; f.next = head; head = f; }
  head = f = null;
  return result;
})()
