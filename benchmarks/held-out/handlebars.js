// Held-out driver: handlebars (MIT) compiles a template and renders it repeatedly.
// Deterministic. Prints one line.
var N = 8000;

var template = Handlebars.compile(
  "{{#each rows}}<tr><td>{{name}}</td><td>{{#if active}}yes{{else}}no{{/if}}</td>" +
  "<td>{{multiply value 3}}</td></tr>{{/each}}"
);
Handlebars.registerHelper("multiply", function (a, b) { return a * b + 1; });

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}
for (var iteration = 0; iteration < N; iteration++) {
  var rows = [];
  for (var i = 0; i < 6; i++) {
    rows.push({ name: "r" + ((iteration + i) % 9), active: (i % 2) === 0, value: i + (iteration % 4) });
  }
  var rendered = template({ rows: rows });
  mix(rendered.length);
  mix(rendered.charCodeAt(rendered.length - 1));
}
print("CHECKSUM " + (hash >>> 0).toString(16));
