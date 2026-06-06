// Derived from: test/language/rest-parameters/no-alias-arguments.js
function f(a, ...rest) {
  arguments[0] = 1;
  if (a !== 3) {
    throw "expected positional parameter not to alias arguments";
  }
  arguments[1] = 2;
  if (rest.join() !== "4,5") {
    throw "expected rest parameter not to alias arguments";
  }
}

f(3, 4, 5);
