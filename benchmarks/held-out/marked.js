// Held-out driver: marked (MIT) renders a generated Markdown document.
// Deterministic. Prints one line.
var N = 100;

var parts = ["# Title\n\n"];
for (var i = 0; i < N; i++) {
  parts.push("## Section " + i + "\n\n");
  parts.push("Paragraph with **bold " + i + "** and `code " + i + "` and [link](https://example.test/" + i + ").\n\n");
  parts.push("* item " + i + "\n* item " + (i + 1) + "\n\n");
  parts.push("> quote " + i + "\n\n");
}

var html = marked.parse(parts.join(""));

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}
mix(html.length);
for (var i = 0; i < html.length; i += 97) {
  mix(html.charCodeAt(i));
}
print("CHECKSUM " + (hash >>> 0).toString(16));
