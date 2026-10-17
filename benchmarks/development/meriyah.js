// Development driver: meriyah (ISC) parses generated modules and walks the AST.
// Deterministic: the source text is a pure function of N. Prints one line.
var N = 1300;

if (typeof structuredClone !== "function") {
  // The pinned QuickJS-NG build has no structuredClone, which meriyah calls
  // when it clones identifiers inside object literals; both engines then see
  // the same AST through one plain-object deep copy.
  globalThis.structuredClone = function clone(value) {
    if (value === null || typeof value !== "object") return value;
    if (Array.isArray(value)) return value.map(clone);
    var copy = {};
    for (var key in value) copy[key] = clone(value[key]);
    return copy;
  };
}

var parts = ["export const acc = 0;\n"];
for (var i = 0; i < N; i++) {
  parts.push("export function fn" + i + "(a, b = " + i + ") { const { x = 1, ...rest } = { x: a, y: b }; return a * " + (i + 1) + " + (rest.y ?? 0) - x / 2; }\n");
  parts.push("export class C" + i + " { #p = " + i + "; static q = /a=/; get v() { return this.#p; } }\n");
  parts.push("acc += fn" + i + "(" + i + ");\n");
}

var program = meriyah.parseModule(parts.join(""), { next: true, ranges: true });

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}
function walk(node) {
  if (node === null || typeof node !== "object") return;
  if (Array.isArray(node)) {
    for (var i = 0; i < node.length; i++) walk(node[i]);
    return;
  }
  if (typeof node.type === "string") {
    mix(node.type.length);
    mix(node.type.charCodeAt(0));
    if (typeof node.end === "number") mix(node.end);
  }
  for (var key in node) {
    if (key === "loc" || key === "range") continue;
    walk(node[key]);
  }
}
walk(program);
mix(program.body.length);
print("CHECKSUM " + (hash >>> 0).toString(16));
