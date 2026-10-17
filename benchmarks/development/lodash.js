// Development driver: lodash (MIT) transforms generated record collections.
// Deterministic: every value is a pure function of N. Prints one line.
var N = 40000;

var records = [];
for (var i = 0; i < N; i++) {
  records.push({
    id: i,
    group: i % 97,
    score: (i * 7919) % 1000,
    tag: "tag" + (i % 13)
  });
}

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}

var total = _.sumBy(records, "score");
var grouped = _.groupBy(records, "group");
var ranked = _.orderBy(records, ["score", "id"], ["desc", "asc"]).slice(0, 64);
var unique = _.uniqBy(records, "group");
var chunked = _.chunk(records, 64);
var flat = _.flatMap(records, function (record) {
  return [record.group, record.score % 10];
});
var counts = _.countBy(records, "tag");

mix(total);
mix(_.keys(grouped).length);
for (var i = 0; i < ranked.length; i++) mix(ranked[i].score + ranked[i].id);
mix(unique.length);
mix(chunked.length + chunked[chunked.length - 1].length);
mix(flat.length);
mix(_.sum(_.values(counts)));

var merged = _.merge({ nested: { a: 1, list: [1, 2] } }, { nested: { b: 2, list: [3] } });
var cloned = _.cloneDeep(ranked);
mix(merged.nested.list.length + merged.nested.list[0] + merged.nested.list[1]);
mix(_.isEqual(cloned, ranked) ? 1 : 0);
mix(_.toPairs(grouped).length);
mix(_.size(grouped[0]));
print("CHECKSUM " + (hash >>> 0).toString(16));
