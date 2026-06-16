// Derived from: test/built-ins/JSON/rawJSON/basic.js
// Derived from: test/built-ins/JSON/rawJSON/returns-expected-object.js
// Derived from: test/built-ins/JSON/rawJSON/illegal-empty-and-start-end-chars.js
// Derived from: test/built-ins/JSON/isRawJSON/basic.js
if (JSON.rawJSON.length !== 1) {
  throw "expected JSON.rawJSON length";
}
if (JSON.isRawJSON.length !== 1) {
  throw "expected JSON.isRawJSON length";
}
if (JSON.stringify(JSON.rawJSON(1.1)) !== "1.1") {
  throw "expected raw number stringification";
}
if (JSON.stringify(JSON.rawJSON(null)) !== "null") {
  throw "expected raw null stringification";
}
if (JSON.stringify(JSON.rawJSON('"foo"')) !== '"foo"') {
  throw "expected raw string stringification";
}
var parsed = JSON.parse(JSON.stringify({ x: JSON.rawJSON(1), y: JSON.rawJSON(2) }));
if (parsed.x !== 1 || parsed.y !== 2) {
  throw "expected raw object property stringification";
}
if (JSON.stringify([JSON.rawJSON(1), JSON.rawJSON(false)]) !== "[1,false]") {
  throw "expected raw array element stringification";
}

var raw = JSON.rawJSON(true);
if (Object.getPrototypeOf(raw) !== null) {
  throw "expected null prototype";
}
if (!Object.hasOwn(raw, "rawJSON")) {
  throw "expected rawJSON own property";
}
if (Object.getOwnPropertyNames(raw).join() !== "rawJSON") {
  throw "expected only rawJSON own property";
}
if (Object.getOwnPropertySymbols(raw).length !== 0) {
  throw "expected no rawJSON symbol properties";
}
if (raw.rawJSON !== "true") {
  throw "expected rawJSON string value";
}
if (!Object.isFrozen(raw)) {
  throw "expected frozen rawJSON object";
}
if (!JSON.isRawJSON(raw)) {
  throw "expected JSON.isRawJSON to recognize raw object";
}
if (JSON.isRawJSON({ rawJSON: "true" })) {
  throw "expected ordinary object rejection";
}

var rejected = false;
try {
  JSON.rawJSON("{}");
} catch (error) {
  rejected = error instanceof SyntaxError;
}
if (!rejected) {
  throw "expected object raw JSON text rejection";
}

for (var text of ["", " 1", "1 ", "\t1", "1\n", "\r1", "1\r"]) {
  rejected = false;
  try {
    JSON.rawJSON(text);
  } catch (error) {
    rejected = error instanceof SyntaxError;
  }
  if (!rejected) {
    throw "expected edge whitespace raw JSON text rejection";
  }
}
