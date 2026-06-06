// Derived from: test/built-ins/Reflect/apply/arguments-list-is-not-array-like-but-still-valid.js
function fn(...args) {
  return args.length + ":" + args[0] + ":" + args[1];
}

if (Reflect.apply(fn, null, [1, 2]) !== "2:1:2") {
  throw "expected rest parameters";
}
