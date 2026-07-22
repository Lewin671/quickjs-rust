// Derived from: test/built-ins/Function/prototype/toString/Function.js
// Derived from: test/built-ins/Function/prototype/toString/GeneratorFunction.js
// Derived from: test/built-ins/Function/prototype/toString/AsyncFunction.js

function sameValue(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(message + ": expected " + expected + ", got " + actual);
  }
}

var GeneratorFunction = (function* () {}).constructor;
var AsyncFunction = (async function () {}).constructor;
var AsyncGeneratorFunction = (async function* () {}).constructor;

sameValue(
  Function("a", "return a;").toString(),
  "function anonymous(a\n) {\nreturn a;\n}",
  "Function constructor source text"
);
sameValue(
  GeneratorFunction("a", "yield a;").toString(),
  "function* anonymous(a\n) {\nyield a;\n}",
  "GeneratorFunction constructor source text"
);
sameValue(
  AsyncFunction("a", "return await a;").toString(),
  "async function anonymous(a\n) {\nreturn await a;\n}",
  "AsyncFunction constructor source text"
);
sameValue(
  AsyncGeneratorFunction("a", "yield await a;").toString(),
  "async function* anonymous(a\n) {\nyield await a;\n}",
  "AsyncGeneratorFunction constructor source text"
);

var scalar = String.fromCodePoint(0xF0000);
var lone = String.fromCharCode(0xD800);
var body = "return '" + scalar + lone + "';";
var source = Function(body).toString();
sameValue(source, "function anonymous(\n) {\n" + body + "\n}", "WTF-16 source text");
sameValue(
  source.slice(source.indexOf("'") + 1, source.lastIndexOf("'")).length,
  3,
  "WTF-16 source contents"
);
