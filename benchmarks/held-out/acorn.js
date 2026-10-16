// Held-out driver: acorn (MIT) parses a generated corpus and walks the AST.
// Deterministic: the source text is a pure function of N. Prints one line.
var N = 1200;

var parts = [];
parts.push("var acc = 0;\n");
for (var i = 0; i < N; i++) {
  parts.push("function fn" + i + "(a, b) { var c = a * " + (i + 1) + " + b; return c - a / 2; }\n");
  parts.push("acc += fn" + i + "(" + i + ", " + (i + 2) + ");\n");
}

var ast = acorn.parse(parts.join(""), { ecmaVersion: 2022 });

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
    mix(node.type.charCodeAt(node.type.length - 1));
  }
  for (var key in node) walk(node[key]);
}
walk(ast);
mix(ast.body.length);
print("CHECKSUM " + (hash >>> 0).toString(16));
