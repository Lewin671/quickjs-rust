// Held-out driver: mustache (MIT) renders a generated template over generated data.
// Deterministic. Prints one line.
var N = 20000;

var template = "{{#items}}[{{index}}:{{name}}:{{value}}]{{/items}}|{{title}}";

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}
for (var iteration = 0; iteration < N; iteration++) {
  var items = [];
  for (var i = 0; i < 8; i++) {
    items.push({ index: i, name: "n" + ((iteration + i) % 7), value: i * 3 + (iteration % 5) });
  }
  var rendered = Mustache.render(template, { items: items, title: "t" + (iteration % 11) });
  mix(rendered.length);
  mix(rendered.charCodeAt(rendered.length - 1));
}
print("CHECKSUM " + (hash >>> 0).toString(16));
