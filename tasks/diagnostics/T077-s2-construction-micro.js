// Construction telemetry controls. Retention forces real materialization.
var mode = scriptArgs[1], n = Number(scriptArgs[2]), keep = [];
function Node(i) { this.left = i; this.right = null; }
class Record { left = 1; right = 2; sum = this.left + this.right; }
for (var i = 0; i < n; i++) {
  if (mode === 'constructor') keep.push(new Node(i));
  else if (mode === 'literal') keep.push({left:i, right:null});
  else if (mode === 'class') keep.push(new Record());
  else if (mode === 'json') keep.push(JSON.parse('{"left":1,"right":2}'));
  else throw new Error('unknown control');
}
var total = 0;
for (var j = 0; j < keep.length; j++) total += mode === 'class' ? keep[j].sum : mode === 'json' ? keep[j].left : keep[j].left;
print('CONTROL', mode, keep.length, total);
