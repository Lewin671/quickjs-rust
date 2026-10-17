// Development driver: papaparse (MIT) parses and re-serializes generated CSV.
// Deterministic: the document is a pure function of N. Prints one line.
var N = 20000;

var rows = ["id,name,notes,value"];
for (var i = 0; i < N; i++) {
  rows.push([
    i,
    "name " + (i % 53),
    i % 7 === 0 ? "quoted, with \"commas\" and\nnewlines" : "plain note " + i,
    ((i * 37) % 1000) / 8
  ].join(i % 5 === 0 ? ";" : ","));
}
var document = rows.join("\n");

var parsed = Papa.parse(document, { delimiter: "", dynamicTyping: true, skipEmptyLines: false });
var unparsed = Papa.unparse(parsed.data, { delimiter: "," });
var reparsed = Papa.parse(unparsed, { dynamicTyping: true, skipEmptyLines: true });
var headed = Papa.parse(document, { header: true, dynamicTyping: true });

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}
mix(parsed.data.length);
mix(parsed.errors.length);
for (var i = 0; i < parsed.data.length; i += 97) {
  var row = parsed.data[i];
  mix(row.length);
  for (var j = 0; j < row.length; j++) {
    var field = row[j];
    if (typeof field === "string") {
      mix(field.length);
      mix(field.charCodeAt(0));
    } else {
      mix(Math.round(field * 100));
    }
  }
}
mix(unparsed.length);
mix(reparsed.data.length);
mix(reparsed.data[10].length);
mix(headed.meta.fields.length);
mix(headed.data.length);
mix(headed.data[3]["name"].length);
print("CHECKSUM " + (hash >>> 0).toString(16));
