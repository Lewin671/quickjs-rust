// Development driver: graphlib (MIT) runs graph algorithms on generated graphs.
// Deterministic: every edge is a pure function of N. Prints one line.
var NODES = 1400;
var SOURCES = 24;
var ALL_NODES = 120;

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}

var graph = new graphlib.Graph({ directed: true });
for (var i = 0; i < NODES; i++) graph.setNode("n" + i, { layer: i % 8, weight: i % 13 });
for (var i = 0; i < NODES; i++) {
  graph.setEdge("n" + i, "n" + ((i * 7 + 1) % NODES), { weight: 1 + (i % 9) });
  if (i % 3 === 0) graph.setEdge("n" + i, "n" + ((i + 17) % NODES), { weight: 2 + (i % 5) });
  if (i % 11 === 0) graph.setEdge("n" + i, "n" + ((i + 101) % NODES), { weight: 5 });
}

var weight = function (edge) { return graph.edge(edge).weight; };
var outEdges = function (node) { return graph.outEdges(node); };

mix(graph.nodeCount());
mix(graph.edgeCount());
mix(graph.sources().length);

for (var source = 0; source < SOURCES; source++) {
  var distances = graphlib.alg.dijkstra(graph, "n" + (source * 53), weight);
  var reachable = 0;
  for (var i = 0; i <= NODES; i += 37) {
    var entry = distances["n" + (i % NODES)];
    if (entry.distance !== Number.POSITIVE_INFINITY) {
      reachable++;
      mix(Math.round(entry.distance * 100));
    }
  }
  mix(reachable);
}

var small = new graphlib.Graph({ directed: true });
for (var i = 0; i < ALL_NODES; i++) {
  small.setNode("s" + i);
  if (i > 0) small.setEdge("s" + (i - 1), "s" + i, { weight: 1 + (i % 4) });
  if (i % 5 === 0) small.setEdge("s" + i, "s" + ((i + 7) % ALL_NODES), { weight: 3 });
}
var smallWeight = function (edge) { return small.edge(edge).weight; };
var smallOut = function (node) { return small.outEdges(node); };
var all = graphlib.alg.dijkstraAll(small, smallWeight, smallOut);
mix(Object.keys(all).length);
mix(Math.round(all["s0"]["s" + (ALL_NODES - 1)].distance * 10));
mix(Math.round(all["s7"]["s30"].distance * 10));

var undirected = new graphlib.Graph({ directed: false });
for (var i = 0; i < NODES; i += 3) {
  undirected.setEdge("u" + i, "u" + ((i + 9) % NODES));
  undirected.setEdge("u" + i, "u" + ((i + 41) % NODES));
}
var components = graphlib.alg.components(undirected);
mix(components.length);
for (var i = 0; i < components.length; i += 7) mix(components[i].length);

var ordered = [];
for (var i = 0; i < 300; i++) ordered.push("d" + (i % 8) + "_" + i);
var dag = new graphlib.Graph({ directed: true });
for (var i = 0; i < ordered.length; i++) {
  dag.setNode(ordered[i]);
  if (i > 0 && i % 8 !== 0) dag.setEdge(ordered[i - 1], ordered[i]);
  if (i >= 8) dag.setEdge(ordered[i - 8], ordered[i]);
}
var sorted = graphlib.alg.topsort(dag);
mix(sorted.length);
mix(sorted[0] === ordered[0] ? 1 : 0);
mix(graphlib.alg.isAcyclic(dag) ? 1 : 0);
mix(graphlib.alg.preorder(dag, [ordered[0]]).length);
print("CHECKSUM " + (hash >>> 0).toString(16));
