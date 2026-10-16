// Held-out driver: esprima (BSD-2-Clause) parses a generated corpus and walks the AST.
// Deterministic. Prints one line.
var N = 12000;

var parts = [];
for (var i = 0; i < N; i++) {
  parts.push("var v" + i + " = " + (i + 1) + " * 3 + (function (x) { return x - " + i + "; })(" + (i + 2) + ");\n");
}

var ast = esprima.parseScript(parts.join(""));

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
  }
  for (var key in node) walk(node[key]);
}
walk(ast);
mix(ast.body.length);
print("CHECKSUM " + (hash >>> 0).toString(16));
