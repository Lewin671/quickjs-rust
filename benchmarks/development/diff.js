// Development driver: diff (jsdiff, BSD-3-Clause) compares generated texts.
// deterministic: every text is a pure function of N. Prints one line.
var N = 160;
var LINES = 12;

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}
function makeText(seed, mutate) {
  var lines = [];
  for (var i = 0; i < LINES; i++) {
    if (mutate && i % 5 === 0) {
      lines.push("inserted line " + seed + " " + i);
    }
    lines.push("line " + ((i + seed) % 11) + " value " + ((i * seed) % 101) + (mutate && i % 3 === 0 ? " changed" : ""));
  }
  return lines.join("\n");
}

for (var i = 0; i < N; i++) {
  var left = makeText(i % 37, false);
  var right = makeText(i % 37, true);

  var chars = Diff.diffChars(left, right);
  var words = Diff.diffWords(left, right);
  var lines = Diff.diffLines(left, right);
  var sentence = Diff.diffSentences(left, right);
  var structured = Diff.structuredPatch("a.txt", "b.txt", left, right);
  var patch = Diff.createPatch("a.txt", left, right);

  mix(chars.length + words.length + lines.length + sentence.length);
  for (var j = 0; j < lines.length; j++) mix(lines[j].count + (lines[j].added ? 1 : 0) + (lines[j].removed ? 1 : 0));
  mix(structured.hunks.length);
  for (var j = 0; j < structured.hunks.length; j++) {
    mix(structured.hunks[j].lines.length);
    mix(structured.hunks[j].oldStart + structured.hunks[j].newStart);
  }
  mix(patch.length);
  mix(patch.charCodeAt(patch.length - 1));
  mix(Diff.applyPatch(left, patch).length);
}
print("CHECKSUM " + (hash >>> 0).toString(16));
