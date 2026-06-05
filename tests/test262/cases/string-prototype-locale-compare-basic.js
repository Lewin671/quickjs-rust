// Derived from: test/built-ins/String/prototype/localeCompare/S15.5.4.9_A1_T1.js
// Derived from: test/built-ins/String/prototype/localeCompare/15.5.4.9_3.js
function sign(value) {
  return value < 0 ? -1 : value > 0 ? 1 : 0;
}

if ('h'.localeCompare('\x68') !== 0) {
  throw 'expected localeCompare equal strings to return zero';
}
if (sign('abc'.localeCompare('abd')) !== -1) {
  throw 'expected localeCompare lower string to return a negative value';
}
if (sign('abd'.localeCompare('abc')) !== 1) {
  throw 'expected localeCompare greater string to return a positive value';
}
if ('undefined'.localeCompare() !== 'undefined'.localeCompare(undefined)) {
  throw 'expected missing localeCompare argument to be treated as undefined';
}
if ('undefined'.localeCompare(undefined) !== 0) {
  throw 'expected undefined argument to stringify to "undefined"';
}
