[
  (function () { function outer(value) { return function () { return eval('value'); }; } return outer(8)(); })(),
  (function () { function outer() { function inner() { eval(''); return () => b; } var read = inner(); eval('var b = 5'); return read(); } return outer(); })(),
  (function () { function outer() { const value = 3; const write = () => eval('value = 4'); try { write(); } catch (error) { return error.name + ':' + value; } } return outer(); })(),
  (function () { function outer() { const write = () => eval('value = 4'); var error; try { write(); } catch (caught) { error = caught.name; } let value = 3; return error + ':' + value; } return outer(); })(),
  (function () { function outer() { eval('var value = 1'); function inner() { eval('var value = 2'); return value; } return inner() + ':' + value; } return outer(); })(),
  (function () { function outer() { eval('function value() { return 1; }'); function inner() { eval('function value() { return 2; }'); return value(); } return inner() + ':' + value(); } return outer(); })(),
  (function () { eval('var introduced = 1'); return eval('delete introduced') + ':' + eval('typeof introduced'); })(),
  (function () { var kept = 1; eval('var kept = 2'); return eval('delete kept') + ':' + kept; })(),
  (function (value) { eval('arguments[0] = 2'); var first = value; value = 4; return first + ':' + eval('arguments[0]'); })(1),
  (function () { var value = 1, object = { value: 0 }; with (object) { eval('var value = 2'); } return value + ':' + object.value; })(),
  (function () { let read = () => arguments; var error; try { read(); } catch (caught) { error = caught.name; } let arguments = 3; return error + ':' + read(); })(),
  (() => { const read = () => typeof arguments; return read(); })(),
  (function () { { let arguments = 'block'; eval('arguments'); } return eval('typeof arguments'); })(),
  (function (value = 1, read = () => value) { var value = 2; return value + ':' + read(); })(),
  (function (value = 1, read = () => value) { eval('var value = 2'); return value + ':' + read(); })()
].join('|')
